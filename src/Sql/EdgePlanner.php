<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Sql;

use Eleph\Gen\WordPress\Ir\EntityDefinition;
use Eleph\Gen\WordPress\Ir\Schema;
use Eleph\Gen\WordPress\Taxonomy\TaxonomyPlacement;

/**
 * Decides where every edge lives, once.
 *
 * Placement is inferred rather than declared: one-to-many puts the key on the many
 * side, many-to-one and one-to-one keep it local, and many-to-many derives a join
 * table. If the framework can work it out, letting a human choose is a chance for two
 * entities to disagree.
 *
 * Taxonomy relationships use domain-owned columns or join tables, just like ordinary edges.
 */
final readonly class EdgePlanner
{
    public function __construct(private Naming $naming = new Naming())
    {
    }

    /**
     * @return array<string, EdgePlacement> Keyed by "Entity.edge".
     */
    public function plan(Schema $schema): array
    {
        $placements = [];

        foreach ($schema->entities as $entity) {
            foreach ($entity->edges as $edge) {
                $target = $schema->entity($edge->to);

                if (null === $target) {
                    continue;
                }

                // Translated once, here. Everything downstream — the placement, the
                // manifest, the adaptor — sees storage's enum, so nothing at run time
                // has to load the spec compiler to know what a relation is.
                $relation = $edge->relation()->forStorage();
                $key = $entity->name . '.' . $edge->name;

                if ($relation->needsJoinTable()) {
                    // A self-referencing edge (Person.friends -> Person) would otherwise
                    // give both sides the same joinColumn($entity->name) — one column,
                    // silently, where the join table needs two. The edge name is unique
                    // per entity already, so it is what disambiguates the target side;
                    // the local side keeps the ordinary name because nothing else on
                    // this table could collide with it.
                    $targetColumnName = $entity->name === $target->name ? $edge->name : $target->name;

                    $placements[$key] = new EdgePlacement(
                        entity: $entity->name,
                        edge: $edge->name,
                        target: $target->name,
                        relation: $relation,
                        table: $this->naming->joinTable($entity->storage->table, $edge->name),
                        localColumn: $this->naming->joinColumn($entity->name),
                        targetColumn: $this->naming->joinColumn($targetColumnName),
                        targetTable: $this->naming->table($target->storage->table),
                    );

                    continue;
                }

                $placements[$key] = $relation->keyIsLocal()
                    ? new EdgePlacement(
                        entity: $entity->name,
                        edge: $edge->name,
                        target: $target->name,
                        relation: $relation,
                        table: $this->naming->table($entity->storage->table),
                        localColumn: $this->naming->column($edge->name) . '_id',
                        targetTable: $this->naming->table($target->storage->table),
                    )
                    : new EdgePlacement(
                        entity: $entity->name,
                        edge: $edge->name,
                        target: $target->name,
                        relation: $relation,
                        table: $this->naming->table($target->storage->table),
                        localColumn: $this->naming->foreignKeyColumn($edge->inverse?->nameFor($entity->name) ?? lcfirst($entity->name)),
                        targetTable: $this->naming->table($target->storage->table),
                    );
            }
        }

        return $placements;
    }

    /**
     * Legacy native relationships are no longer generated; domain edges use plan().
     *
     * @return array<string, TaxonomyPlacement> Keyed by "Entity.edge".
     */
    public function planTaxonomies(Schema $schema): array
    {
        return [];
    }

    /**
     * Whether a pattern has marked this entity's rows as WordPress taxonomy terms
     * rather than rows in a table of its own.
     *
     * Read from pattern configuration, the same extension point `visibility` and
     * `adminMenu` already use for post types — the core schema never learns what a
     * taxonomy is, and this package stays the only thing that does.
     */
    public static function isTaxonomy(EntityDefinition $entity): bool
    {
        return true === $entity->configured('taxonomy', false);
    }

    /**
     * Whether a pattern has marked this entity's rows as WordPress accounts rather
     * than rows in a table of its own.
     *
     * The same extension point `taxonomy` uses, and for the same reason: the core
     * schema never learns what `wp_users` is, and this package stays the only thing
     * that does. Unlike a taxonomy, an account-backed entity needs no edge placement
     * of its own — an edge pointing at one is an ordinary column or join table storing
     * an id that happens to be a `wp_users.ID`, which every other relation already
     * knows how to place.
     */
    public static function isAccount(EntityDefinition $entity): bool
    {
        return true === $entity->configured('account', false);
    }
}

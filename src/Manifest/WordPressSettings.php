<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Manifest;

use Eleph\Gen\WordPress\Ir\EntityDefinition;
use Eleph\Gen\WordPress\Ir\Schema;
use Eleph\Gen\WordPress\Sql\EdgePlanner;

final class WordPressSettings
{
    public static function enabled(Schema $schema, EntityDefinition $entity, string $key): bool
    {
        return true === ($entity->integrations['wordpress'][$key] ?? $schema->project->integrations['wordpress'][$key] ?? ('adminTemplates' === $key));
    }

    public static function linked(Schema $schema, EntityDefinition $entity): bool
    {
        return !EdgePlanner::isTaxonomy($entity) && !EdgePlanner::isAccount($entity)
            && 'wordpress' === $entity->storage->driver && null !== $entity->storage->handle
            && self::enabled($schema, $entity, 'linkPosts');
    }

    public static function admin(Schema $schema, EntityDefinition $entity): bool
    {
        return !EdgePlanner::isTaxonomy($entity) && !EdgePlanner::isAccount($entity)
            && self::enabled($schema, $entity, 'adminTemplates')
            && true === $entity->configured('showInAdmin', true);
    }
}

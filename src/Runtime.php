<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress;

/**
 * The runtime classes the compiled manifest refers to, as names rather than as imports.
 *
 * The storage manifest this builder writes is `return new StorageManifest(tables: [...
 * new TableSchema(...) ...])` — real constructor calls, so the file type-checks in the
 * project that loads it and a diff reads as a description of the schema. Writing those
 * calls means naming the classes, but this repository does not have them to import:
 * `elephentity/wordpress` is not a dependency here, by design (docs/BUILDERS.md — "It
 * depends on nothing of Elephentity's"), so every name below is a string, embedded into
 * generated text, never a `use` this file's own code resolves.
 *
 * If one of these is renamed in `elephentity/wordpress`, nothing here fails — this
 * builder still emits the old name, and the project that installed the new runtime
 * fails to load the manifest at boot. Regenerating `examples/clog` in `elephentity` is
 * what catches it.
 */
final class Runtime
{
    public const STORAGE_MANIFEST = 'Eleph\WordPress\Manifest\StorageManifest';
    public const ACCOUNT_FIELDS = 'Eleph\WordPress\Account\AccountFields';
    public const COLUMN = 'Eleph\WordPress\Sql\Column';
    public const EDGE_PLACEMENT = 'Eleph\WordPress\Sql\EdgePlacement';
    public const INDEX = 'Eleph\WordPress\Sql\Index';
    public const TABLE_SCHEMA = 'Eleph\WordPress\Sql\TableSchema';
    public const TAXONOMY_PLACEMENT = 'Eleph\WordPress\Taxonomy\TaxonomyPlacement';
    public const RELATION_KIND = 'Eleph\Runtime\Storage\RelationKind';
}

# elephentity-codegen-wordpress

The WordPress builder for [Elephentity](https://github.com/hsimah-services/elephentity).

It reads one JSON request on stdin — the compiled spec, plus the target's configuration
— and writes one JSON response on stdout: a path and a body per file. It never touches
the filesystem. Signing and writing happen in
[elephentity-codegen](https://github.com/hsimah-services/elephentity-codegen), after this exits.

```bash
echo '{"elephentity":1,"irVersion":"1.1","target":"wordpress","config":{},
       "outputDirectory":"out","schema":{}}' | ./bin/eleph-gen-wordpress
```

That fails on the empty schema, which is the point: it should be obvious how.

## Installing it

```bash
composer require --dev elephentity/codegen-wordpress
```

Then name it in `eleph.json`:

```json
{
  "targets": {
    "wordpress": {
      "builder": "vendor/bin/eleph-gen-wordpress",
      "output": "generated/wordpress"
    }
  }
}
```

Only where the project's `project.yml` names `driver: wordpress` — configuring this
target for another driver is a response with an error, not a file.

## What it provides

- The `wordpress` driver, and the rules it imposes on `storage.handle` — a 20-character,
  lowercase-slug limit — so a spec is validated against this driver's own declaration
  rather than one hardcoded into the compiler.
- The `Taxonomy` pattern (`resources/patterns/Taxonomy.yml`): a project may `use:` it
  with nothing under `spec/patterns/`.
- The physical schema `elephentity/wordpress`'s adaptor loads at boot: table
  definitions, edge placements, post type and taxonomy registration arguments.

## It depends on nothing of Elephentity's

Not the compiler, not the runtime, not the orchestrator. The IR value objects in
`src/Ir` are a copy; the physical-schema shapes (`src/Sql`, `src/Manifest`) are a
second, independent copy of the same shapes the runtime holds; and the runtime classes
the exported manifest refers to are strings in `src/Runtime.php` rather than imports.
That is deliberate: a builder that had to `composer require` the framework it generates
for would be a builder no other language could write. The version gate is what holds
the copies in step — a mismatch is a refusal, never a silent misread.

## Working on it

There is no local PHP; everything runs in a container:

```bash
./tools/php composer ci          # style, static analysis, tests
./tools/php vendor/bin/phpunit --filter GoldenTest
```

PHPStan runs at **level max** with no baseline exclusions.

## The golden fixtures

`tests/fixtures/golden/*/` holds a committed request and the exact response it
produces, asserted byte for byte through the real binary. When a deliberate change
moves them, regenerate and read the diff — it is the clearest description available of
what the change did to every project's generated tree.

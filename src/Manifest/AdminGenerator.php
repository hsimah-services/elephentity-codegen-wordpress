<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Manifest;

use Eleph\Gen\WordPress\Ir\Schema;

/** Independent PHP reference for the generated admin manifests and templates. */
final class AdminGenerator
{
    /** @return array<string, string> */
    public function generate(Schema $schema): array
    {
        $files = [];
        $pages = [];

        foreach ($schema->entities as $entity) {
            if (!WordPressSettings::admin($schema, $entity)) {
                continue;
            }

            $name = $entity->name;
            $label = $entity->configured('label');
            $label = is_string($label) ? $label : $this->humanise($name);
            $plural = $entity->configured('pluralLabel');
            $plural = is_string($plural) ? $plural : $this->plural($label);
            $fields = ['id' => 'ID'];

            foreach ($entity->fields as $field) {
                $fields[$field->name] = $this->humanise(ucfirst($field->name));
            }

            $rendered = $this->render($fields, 1);

            foreach (['list' => [$plural, 'listing'], 'detail' => [$label, 'detail']] as $kind => [$title, $method]) {
                $files["admin/$name/$kind.php"] = sprintf(
                    "use Eleph\\WordPress\\Admin\\View;\n\nreturn static function (View \$view, array \$data): void {\n    /** @var array<string, mixed> \$data */\n    \$view->%s(%s, %s, \$data);\n};\n",
                    $method,
                    var_export($title, true),
                    $rendered,
                );
            }

            $pages[$name] = [
                'label' => $plural,
                'slug' => 'eleph-' . $entity->storage->table,
                'parent' => $entity->configured('adminMenu'),
                'list' => "admin/$name/list.php",
                'detail' => "admin/$name/detail.php",
            ];
        }

        ksort($pages);

        return ['admin-pages.php' => 'return ' . $this->render($pages) . ";\n", ...$files];
    }

    /** @param array<array-key, mixed> $value */
    private function render(array $value, int $depth = 0): string
    {
        if ([] === $value) {
            return '[]';
        }

        $lines = [];

        foreach ($value as $key => $item) {
            $lines[] = str_repeat('    ', $depth + 1) . var_export($key, true) . ' => '
                . (is_array($item) ? $this->render($item, $depth + 1) : (null === $item ? 'null' : var_export($item, true))) . ',';
        }

        return "[\n" . implode("\n", $lines) . "\n" . str_repeat('    ', $depth) . ']';
    }

    private function humanise(string $name): string
    {
        return trim((string) preg_replace('/(?<!^)[A-Z]/', ' $0', $name));
    }

    private function plural(string $label): string
    {
        if (1 === preg_match('/(s|x|z|ch|sh)$/i', $label)) {
            return $label . 'es';
        }

        return 1 === preg_match('/[^aeiou]y$/i', $label) ? substr($label, 0, -1) . 'ies' : $label . 's';
    }
}

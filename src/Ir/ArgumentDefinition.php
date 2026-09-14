<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Ir;

final readonly class ArgumentDefinition
{
    public function __construct(
        public string $name,
        public TypeReference $type,
        public bool $nullable = false,
    ) {
    }
}

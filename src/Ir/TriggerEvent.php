<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Ir;

enum TriggerEvent: string
{
    case Create = 'create';
    case Update = 'update';
    case Delete = 'delete';
}

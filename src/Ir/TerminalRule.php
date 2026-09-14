<?php

declare(strict_types=1);

namespace Eleph\Gen\WordPress\Ir;

enum TerminalRule: string
{
    case Allow = 'allow';
    case Deny = 'deny';
}

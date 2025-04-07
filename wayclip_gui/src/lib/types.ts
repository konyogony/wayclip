import { IconType } from '@vertisanpro/react-icons';

export enum groups {
    main = 'Main',
    library = 'Library',
    settings = 'Settings',
}

export interface Route {
    name: string;
    path: string;
    icon: IconType;
    group: groups;
}

export enum categories {
    general = 'General',
    ui = 'UI',
    shortcuts = 'Shortcuts',
}

export interface Setting {
    name: string;
    description?: string;
    tooltip?: string;
    type: 'string' | 'select' | 'boolean' | 'number';
    defaultValue?: string | number | boolean;
    storageKey: string;
    category: categories;
}

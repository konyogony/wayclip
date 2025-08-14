import { IconType } from '@vertisanpro/react-icons';
import { createContext } from 'react';
import { JSX } from 'react';

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
    element: JSX.Element;
}

export enum categories {
    general = 'General',
    ui = 'UI',
    shortcuts = 'Shortcuts',
    audio = 'Audio',
}

export interface Setting {
    name: string;
    description?: string;
    tooltip?: string;
    type: 'string' | 'select' | 'boolean' | 'number';
    options?: (string | number)[];
    defaultValue: JsonValue;
    currentValue?: JsonValue;
    storageKey: string;
    category: categories;
}

// JSON types (got from somewhere)
export interface JsonArray extends Array<JsonValue> {}
export type JsonValue = string | number | boolean | null | JsonObject | JsonArray;
export type JsonObject = { [Key in string]?: JsonValue };

export type SidebarContextType = {
    isOpen: boolean;
    toggleSidebar: () => void;
};

export const SidebarContext = createContext<SidebarContextType | undefined>(undefined);

export interface AudioDevice {
    name: string;
    id: number;
    node_name: string;
}

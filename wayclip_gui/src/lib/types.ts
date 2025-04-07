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

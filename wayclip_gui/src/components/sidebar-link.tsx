import { Route } from '@/lib/types';
import { cn } from '@/lib/utils';
import { NavLink } from 'react-router';

export const SidebarLink = ({ name, path, icon: Icon, isSidebarOpened }: Route & { isSidebarOpened: boolean }) => {
    return (
        <NavLink
            to={path}
            className={({ isActive }) =>
                cn(
                    'w-full flex items-center gap-3 px-3 py-3 rounded-lg text-sm transition-all duration-200 relative',
                    isActive ? 'bg-zinc-800 text-white' : 'text-zinc-400 hover:text-white hover:bg-zinc-800/50',
                )
            }
        >
            <Icon size={16} className='flex-shrink-0' />
            <span
                className={cn(
                    'absolute left-10 w-32',
                    !isSidebarOpened && 'opacity-0 pointer-events-none cursor-none transition-all duration-300',
                )}
            >
                {name}
            </span>
        </NavLink>
    );
};

import { Route } from '@/lib/types';
import { NavLink } from 'react-router';

export const SidebarLink = ({ name, path, icon: Icon }: Route) => {
    return (
        <NavLink
            to={path}
            className='mt-auto w-full gap-2.5 flex-row flex px-3 py-1 group rounded-lg items-center hover:bg-zinc-800/80 transition-all duration-200 ease-in-out border-2
                border-transparent aria-[current=page]:bg-indigo-700/80 aria-[current=page]:text-zinc-100 text-zinc-400 group-hover:text-zinc-200'
        >
            <Icon size={16} className='my-auto' />
            <span className='my-auto font-medium'>{name}</span>
        </NavLink>
    );
};

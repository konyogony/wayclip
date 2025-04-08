import { FaMagnifyingGlass } from '@vertisanpro/react-icons/fa6';
import { useState } from 'react';
import { FiSidebar } from '@vertisanpro/react-icons/fi';
import { cn } from '@/lib/utils';
import { Route } from '@/lib/types';
import { groups } from '@/lib/types';
import { SidebarLink } from '@/components/sidebar-link';
import { InfoCard } from '@/components/info-card';

interface SidebarProps {
    routes: Route[];
}

export const Sidebar = ({ routes }: SidebarProps) => {
    const [isSidebarOpen, setIsSidebarOpen] = useState(true);
    return (
        <>
            <button
                className={cn(
                    'flex flex-row items-center cursor-pointer gap-2 w-fit h-fit justify-center transition-all duration-200 ease-in-out rounded-lg p-2 absolute top-4 left-4 z-30',
                    isSidebarOpen ? 'hover:bg-zinc-800/80' : 'hover:bg-zinc-900/50',
                )}
                onClick={() => setIsSidebarOpen((prev) => !prev)}
            >
                <FiSidebar size={18} />
            </button>
            <div
                className={cn(
                    'flex relative flex-col gap-6 p-4 shrink-0  bg-zinc-900 transition-all duration-200 ease-in-out h-screen shadow-lg z-20',
                    isSidebarOpen ? 'w-1/6 3xl:w-1/8' : 'w-0 overflow-hidden opacity-0 pointer-events-none cursor-none',
                )}
            >
                <span className='text-2xl font-bold mb-2 mx-auto'>Wayclip</span>
                <div className='w-full flex flex-row items-center gap-2 bg-zinc-800/50 text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400'>
                    <FaMagnifyingGlass size={18} className='text-zinc-400' />
                    <input type='text' className='w-full focus:ring-0 focus:outline-0' placeholder='Search...' />
                </div>
                {Object.values(groups).map((group) => (
                    <div className='flex flex-col gap-2' key={group}>
                        <span className='text-zinc-400 text-sm font-semibold'>{group}</span>
                        {routes
                            .filter((route) => route.group === group)
                            .map((v, i) => (
                                <SidebarLink key={i} {...v} />
                            ))}
                    </div>
                ))}
                <InfoCard />
            </div>
        </>
    );
};

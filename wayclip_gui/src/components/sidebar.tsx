import { cn } from '@/lib/utils';
import { Route } from '@/lib/types';
import { groups } from '@/lib/types';
import { SidebarLink } from '@/components/sidebar-link';

interface SidebarProps {
    routes: Route[];
    isOpen: boolean;
}

export const StatusComponent = ({
    status,
    isSidebarOpened,
    className,
}: {
    status: string;
    isSidebarOpened: boolean;
    className?: string;
}) => {
    return (
        <div
            className={cn(
                'group w-fit flex border border-zinc-800 group-hover:border-zinc-600 rounded-xl px-3 py-1.5 font-mono text-sm gap-2 items-center  transition-all duration-200',
                status === 'Active' ? 'text-zinc-300' : 'text-zinc-400',
                className,
                !isSidebarOpened && 'opacity-0 pointer-events-none select-none cursor-none',
            )}
        >
            <span>Daemon status: {status}</span>

            <div
                className={cn(
                    'flex size-2 rounded-full relative justify-center items-center  -translate-y-[1px] ml-auto',
                    status === 'Inactive' ? 'bg-zinc-500' : 'bg-green-400',
                )}
            >
                {status === 'Active' && <div className={cn('flex size-2 rounded-full', 'bg-green-400 animate-ping')} />}
            </div>
        </div>
    );
};

export const Sidebar = ({ routes, isOpen }: SidebarProps) => {
    return (
        <div
            className={cn(
                'flex relative flex-col gap-6 shrink-0 transition-all duration-200 ease-in-out h-full shadow-lg z-20 pt-4',
                isOpen
                    ? 'w-1/5 xl:w-1/5 3xl:w-1/8'
                    : 'w-0 opacity-0 cursor-none pointer-events-none select-none overflow-hidden',
            )}
        >
            <StatusComponent status={'Active'} isSidebarOpened={isOpen} className='mx-6 w-10/12' />
            <div className={cn('absolute className flex flex-col gap-6 w-full top-16')}>
                {Object.values(groups).map((group) => (
                    <div className='flex flex-col gap-1.5 mx-6 relative' key={group}>
                        <span
                            className={cn(
                                'text-xs font-medium text-zinc-400 uppercase tracking-wider px-3 py-2 w-fit transition-all duration-200',
                                !isOpen && 'opacity-0 pointer-events-none cursor-none',
                            )}
                        >
                            {group}
                        </span>
                        <div className='flex gap-1.5 flex-col w-full relative'>
                            {routes
                                .filter((route) => route.group === group)
                                .map((v, i) => (
                                    <SidebarLink key={i} {...v} isSidebarOpened={isOpen} />
                                ))}
                        </div>
                    </div>
                ))}
            </div>
            <span className='mt-auto text-sm text-zinc-400 w-full text-center border-t border-zinc-800 pb-2 pt-3 font-mono relative'>
                Wayclip App v{import.meta.env.PACKAGE_VERSION}
            </span>
        </div>
    );
};

import { Outlet } from 'react-router';
import { FaGear, FaHouse, FaHeart, FaBell, FaClapperboard } from '@vertisanpro/react-icons/fa6';
import { groups, Route } from '@/lib/types';
import { Sidebar } from '@/components/sidebar';

const routes: Route[] = [
    {
        name: 'Home',
        path: '/',
        icon: FaHouse,
        group: groups.main,
    },
    {
        name: 'All Clips',
        path: '/all-clips',
        icon: FaClapperboard,
        group: groups.library,
    },
    {
        name: 'Liked Clips',
        path: '/liked-clips',
        icon: FaHeart,
        group: groups.library,
    },
    {
        name: 'Notifications',
        path: '/notifications',
        icon: FaBell,
        group: groups.settings,
    },
    {
        name: 'Settings',
        path: '/settings',
        icon: FaGear,
        group: groups.settings,
    },
];

const MainLayout = () => {
    return (
        <main className='flex flex-row min-h-screen'>
            <Sidebar routes={routes} />
            <div className='flex flex-col items-center w-full'>
                <Outlet />
            </div>
        </main>
    );
};

export default MainLayout;

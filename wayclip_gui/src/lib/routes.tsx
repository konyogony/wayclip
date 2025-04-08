import { FaBell, FaClapperboard, FaGear, FaHeart, FaHouse } from '@vertisanpro/react-icons/fa6';
import { Route, groups } from '@/lib/types';
import Home from '@/pages/home';
import AllClips from '@/pages/all-clips';
import LikedClips from '@/pages/liked-clips';
import Notifications from '@/pages/notifications';
import Settings from '@/pages/settings';

export const routes: Route[] = [
    {
        name: 'Home',
        path: '/',
        icon: FaHouse,
        group: groups.main,
        element: <Home />,
    },
    {
        name: 'All Clips',
        path: '/all-clips',
        icon: FaClapperboard,
        group: groups.library,
        element: <AllClips />,
    },
    {
        name: 'Liked Clips',
        path: '/liked-clips',
        icon: FaHeart,
        group: groups.library,
        element: <LikedClips />,
    },
    {
        name: 'Notifications',
        path: '/notifications',
        icon: FaBell,
        group: groups.settings,
        element: <Notifications />,
    },
    {
        name: 'Settings',
        path: '/settings',
        icon: FaGear,
        group: groups.settings,
        element: <Settings />,
    },
];

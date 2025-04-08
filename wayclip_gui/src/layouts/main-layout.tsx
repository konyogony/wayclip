import { Outlet } from 'react-router';
import { routes } from '@/lib/routes';
import { Sidebar } from '@/components/sidebar';
import Toaster from '@/components/toaster';

const MainLayout = () => {
    return (
        <main className='flex flex-row min-h-screen'>
            <Sidebar routes={routes} />
            <div className='flex flex-col items-center w-full'>
                <Outlet />
            </div>
            <Toaster />
        </main>
    );
};

export default MainLayout;

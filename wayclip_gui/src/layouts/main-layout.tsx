import { Outlet } from 'react-router';

const MainLayout = () => {
    return (
        <main className='flex flex-col min-h-screen'>
            <Outlet />
        </main>
    );
};

export default MainLayout;

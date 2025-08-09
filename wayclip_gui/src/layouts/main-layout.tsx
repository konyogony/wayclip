import { Outlet } from 'react-router';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { routes } from '@/lib/routes';
import { RxArrowLeft, RxArrowRight, RxCross2 } from '@vertisanpro/react-icons/rx';
import { Sidebar } from '@/components/sidebar';
import Toaster from '@/components/toaster';
import Logo from '../../src-tauri/icons/logo3.png';

const appWindow = getCurrentWindow();

const MainLayout = () => {
    return (
        <main className='flex flex-col h-screen'>
            <div
                className='flex-shrink-0 py-2 w-full flex flex-row gap-2 select-none px-2 items-center'
                data-tauri-drag-region
            >
                <span className='flex text-[22px] font-bold ml-3 items-center'>
                    <img src={Logo} alt='Wayclip Logo' className='size-10 text-white' />
                    Wayclip App
                </span>
                <span className='flex flex-row gap-1 items-center'>
                    <button className='hover:bg-zinc-600/50 size-6 rounded-full flex items-center justify-center'>
                        <RxArrowLeft size={14} />
                    </button>
                    <button className='hover:bg-zinc-600/50 size-6 rounded-full flex items-center justify-center'>
                        <RxArrowRight size={14} />
                    </button>
                </span>
                <button
                    onClick={() => appWindow.close()}
                    className='hover:bg-zinc-600/50 size-6 rounded-full flex items-center justify-center ml-auto'
                    id='titlebar-close'
                    title='close'
                >
                    <RxCross2 size={14} />
                </button>
            </div>
            <div className='flex flex-row flex-1 overflow-hidden'>
                <Sidebar routes={routes} />
                <div className='flex flex-col items-center w-full rounded-tl-2xl bg-zinc-900 overflow-y-auto border-l border-t border-zinc-700'>
                    <Outlet />
                </div>
            </div>
            <Toaster />
        </main>
    );
};

export default MainLayout;

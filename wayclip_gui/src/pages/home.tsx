import { cn } from '@/lib/utils';
import { FiSidebar } from '@vertisanpro/react-icons/fi';
import { useSidebar } from '@/layouts/main-layout';

const Home = () => {
    const { toggleSidebar } = useSidebar();
    return (
        <div className='flex flex-col w-full h-full relative'>
            <div className='flex items-center gap-3 w-full border-b border-zinc-800 py-4 px-10'>
                <button
                    className={cn(
                        'flex flex-row items-center cursor-pointer gap-2 w-fit h-fit justify-center transition-all duration-200 ease-in-out rounded-lg p-2 z-30 hover:bg-zinc-800/50',
                    )}
                    onClick={toggleSidebar}
                >
                    <FiSidebar size={18} />
                </button>
                <div className='w-[1px] h-8 mr-1 bg-zinc-800' />
                <h1 className='text-2xl font-bold'>Home</h1>
            </div>
        </div>
    );
};

export default Home;

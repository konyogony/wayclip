import { motion } from 'motion/react';
import { cn } from '@/lib/utils';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/animate-ui/radix/dropdown-menu';
import { Button } from '@/components/ui/button';
import { FiMoreHorizontal, FiSidebar } from '@vertisanpro/react-icons/fi';
import { useSidebar } from '@/layouts/main-layout';

const Home = () => {
    const { toggleSidebar } = useSidebar();
    return (
        <div className='flex flex-col w-full h-full'>
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
            <DropdownMenu>
                <DropdownMenuTrigger asChild className='w-fit'>
                    <Button
                        size='sm'
                        variant='ghost'
                        className='text-zinc-400 hover:text-white hover:bg-zinc-700/60 w-fit'
                        asChild
                    >
                        <motion.button whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }} className='w-fit'>
                            <FiMoreHorizontal className='w-4 h-4' />
                        </motion.button>
                    </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align='start' side='bottom'>
                    <DropdownMenuLabel>Actions</DropdownMenuLabel>
                    <DropdownMenuGroup>
                        <DropdownMenuItem>Rename</DropdownMenuItem>
                        <DropdownMenuItem variant='destructive'>
                            <span>Delete</span>
                        </DropdownMenuItem>
                        <DropdownMenuItem>View</DropdownMenuItem>
                        <DropdownMenuItem>Open folder</DropdownMenuItem>
                    </DropdownMenuGroup>
                    <DropdownMenuSeparator />
                    <DropdownMenuGroup>
                        <DropdownMenuLabel>Info</DropdownMenuLabel>
                        <DropdownMenuItem disabled>Size:</DropdownMenuItem>
                        <DropdownMenuItem disabled>Duration:</DropdownMenuItem>
                        <DropdownMenuItem disabled>Created:</DropdownMenuItem>
                        <DropdownMenuItem disabled>Modified:</DropdownMenuItem>
                        <DropdownMenuItem disabled>Path:</DropdownMenuItem>
                    </DropdownMenuGroup>
                </DropdownMenuContent>
            </DropdownMenu>
        </div>
    );
};

export default Home;

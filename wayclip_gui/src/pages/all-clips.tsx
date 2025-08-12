import { useEffect, useState } from 'react';
import { cn } from '@/lib/utils';
import { useSidebar } from '@/layouts/main-layout';
import { invoke } from '@tauri-apps/api/core';
import { ClipPreview } from '@/components/clip-preview';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { FaMagnifyingGlass, FaSort } from '@vertisanpro/react-icons/fa6';
import { FiFilter, FiSidebar } from '@vertisanpro/react-icons/fi';

export interface ClipData {
    name: string;
    path: string;
    length: number;
    created_at: string;
    updated_at: string;
    size: number;
    tags: {
        name: string;
        color: string;
    }[];
    liked: boolean;
}

const AllClips = () => {
    const [clips, setClips] = useState<ClipData[]>([]);
    const [searchQuery, setSearchQuery] = useState('');
    const { toggleSidebar } = useSidebar();

    useEffect(() => {
        const fetchData = async () => {
            const clips = await invoke('pull_clips').catch((e) => console.error(e));
            setClips(clips as ClipData[]);
        };
        fetchData();
        console.log(clips);
    }, []);

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
                <h1 className='text-2xl font-bold'>All Clips</h1>
                {clips.length > 0 && <span className='text-zinc-300 text-sm'>{clips.length} clips </span>}
                <div className='relative ml-auto'>
                    <FaMagnifyingGlass className='absolute left-3 top-1/2 transform -translate-y-1/2 text-zinc-400 w-4 h-4' />
                    <Input
                        placeholder='Search...'
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className='pl-10 w-80 bg-zinc-900 border-zinc-700 text-white placeholder:text-zinc-400 focus:border-zinc-600'
                    />
                </div>
                <Button variant='ghost' size='icon' className='text-zinc-400 hover:text-white hover:bg-zinc-800'>
                    <FiFilter className='w-4 h-4' />
                </Button>
                <Button variant='ghost' size='icon' className='text-zinc-400 hover:text-white hover:bg-zinc-800'>
                    <FaSort className='w-4 h-4' />
                </Button>
            </div>

            {clips.length > 0 ? (
                <div className='w-full h-fit grid-cols-1 md:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4 grid gap-6 p-8'>
                    {clips.slice(0, 1).map((v, i) => (
                        <ClipPreview {...v} key={i} />
                    ))}
                </div>
            ) : (
                <div className='flex flex-col mx-auto my-auto items-center'>
                    <span className='text-lg'>No clips found</span>
                    <span>Seems like you dont have any clips saved. They will appear here</span>
                </div>
            )}
        </div>
    );
};

export default AllClips;

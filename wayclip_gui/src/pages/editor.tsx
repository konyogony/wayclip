import { useSidebar } from '@/hooks/sidebar';
import { FiSidebar } from '@vertisanpro/react-icons/fi';
import { useEffect, useState } from 'react';
import { useParams } from 'react-router';
import { ClipData, PaginatedClips } from './all-clips';
import { invoke } from '@tauri-apps/api/core';

const Editor = () => {
    const { toggleSidebar } = useSidebar();
    const { name } = useParams();
    const [clipData, setClipData] = useState<ClipData>();

    useEffect(() => {
        const fetchClips = async (page: number, searchQuery: string): Promise<PaginatedClips> => {
            const data = await invoke<PaginatedClips>('pull_clips', {
                page: page,
                pageSize: 1,
                searchQuery: searchQuery || null,
            });
            setClipData(data.clips[0]);
        };
        name && fetchClips(1, name);
        console.log(clipData);
    }, []);
    return (
        <div className='flex flex-col w-full h-full'>
            <div className='flex items-center gap-3 w-full border-b border-zinc-800 py-4 px-10 flex-shrink-0'>
                <button
                    className='flex flex-row items-center cursor-pointer gap-2 w-fit h-fit justify-center transition-all duration-200 ease-in-out rounded-lg p-2 z-30 hover:bg-zinc-800/50'
                    onClick={toggleSidebar}
                >
                    <FiSidebar size={18} />
                </button>
                <div className='w-[1px] h-8 mr-1 bg-zinc-800' />
                <h1 className='text-2xl font-bold'>Clip editor</h1>
            </div>
            <video src={''} className='w-full h-full' controls />
        </div>
    );
};

export default Editor;

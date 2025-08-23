import { useSidebar } from '@/hooks/sidebar';
import { convertName } from '@/lib/lib';
import { FiSidebar } from '@vertisanpro/react-icons/fi';
import { useEffect, useState, useRef } from 'react';
import { useParams } from 'react-router';
import { ClipData, PaginatedClips } from './all-clips';
import { invoke } from '@tauri-apps/api/core';
import { getVideo } from '@/lib/lib';

const Editor = () => {
    const { toggleSidebar } = useSidebar();
    const { name } = useParams();
    const [clipData, setClipData] = useState<ClipData>();
    const [src, setSrc] = useState<string | null>(null);
    const videoRef = useRef<HTMLVideoElement>(null);

    useEffect(() => {
        const fetchClips = async (page: number, searchQuery: string) => {
            console.log(searchQuery);
            const data = await invoke<PaginatedClips>('pull_clips', {
                page: page,
                pageSize: 1,
                searchQuery: searchQuery.replace('.mp4', ''),
            });
            console.log(data.clips[0]);
            if (data.clips[0]) {
                setClipData(data.clips[0]);
                getVideo(data.clips[0].path)
                    .then((src) => {
                        console.log(src);
                        setSrc(src);
                    })
                    .catch((e) => console.error(e));
            }
        };
        name && fetchClips(1, name);
    }, []);

    const handleVideoError = (e: React.SyntheticEvent<HTMLVideoElement, Event>) => {
        console.error(`Video Error`, e.currentTarget.error);
    };

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
            {clipData && (
                <div className='flex flex-col p-8 gap-4'>
                    <span className='text-2xl font-bold w-full border-b border-zinc-800'>
                        {convertName(clipData.name, 'storeToDisplay')}
                    </span>
                    <div className='flex flex-row gap-2 w-full'>
                        <div className='flex aspect-video w-3/4 bg-zinc-700 rounded-xl overflow-clip'>
                            {src && (
                                <video
                                    src={src}
                                    ref={videoRef}
                                    onError={handleVideoError}
                                    className='w-full h-full'
                                    controls
                                />
                            )}
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
};

export default Editor;

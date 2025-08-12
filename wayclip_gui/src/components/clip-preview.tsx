import { ClipData } from '@/pages/all-clips';
import { cn } from '@/lib/utils';
import { memo, useMemo, useState } from 'react';
import { FiPlay, FiHeart, FiMoreHorizontal, FiTrash } from '@vertisanpro/react-icons/fi';
import { Button } from '@/components/ui/button';

const convertLength = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const paddedSecs = secs.toString().padStart(2, '0');
    return `${mins}:${paddedSecs}`;
};

const covertTime = (ts: string): string => {
    const date = new Date(ts);

    const time = date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
    const month = date.toLocaleString('en-US', { month: 'long' });
    const day = date.getDate();
    const year = date.getFullYear();

    return `${time}, ${month} ${day}, ${year}`;
};

const convertSize = (bytes: number, decimals = 2): string => {
    if (bytes === 0) return '0 Bytes';

    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];

    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const value = bytes / Math.pow(k, i);

    return `${parseFloat(value.toFixed(decimals))} ${sizes[i]}`;
};

const changeName = (path: string, newName: string) => {
    console.log(`changed name of ${path} to ${newName}`);
};

const toggleLike = (name: string, prev: boolean) => {
    console.log(`changed ${name} from ${prev} to ${!prev}`);
};

const ClipPreviewComponent = ({ name, length, path, created_at, liked, size, tags }: ClipData) => {
    const [isLiked, setIsLiked] = useState(liked);

    const duration = useMemo(() => {
        return convertLength(length);
    }, []);

    const timestamp = useMemo(() => {
        return covertTime(created_at);
    }, []);

    const fileSize = useMemo(() => {
        return convertSize(size);
    }, []);
    return (
        <div className='group flex flex-col rounded-2xl w-full h-fit bg-[#18181b] overflow-clip gap-1 shadow-sm hover:border-zinc-500 transition-all duration-300 border-zinc-800 border'>
            <div className='relative aspect-video bg-zinc-800'>
                <div className='absolute inset-0 flex items-center justify-center'>
                    <div className='w-12 h-12 bg-zinc-700 rounded-full flex items-center justify-center opacity-60 group-hover:opacity-100 transition-opacity'>
                        <FiPlay className='w-5 h-5 text-white ml-0.5' />
                    </div>
                </div>

                <div className='absolute bottom-2 right-2 bg-black/80 text-white text-xs px-2 py-1 rounded'>
                    {duration}
                </div>

                {tags && (
                    <div className='absolute bottom-2 left-2 flex flex-row gap-1 text-white text-xs'>
                        {tags.map((v, i) => (
                            <div
                                style={{
                                    backgroundColor: v.color,
                                }}
                                className='px-1.5 py-0.5 rounded-full'
                                key={i}
                            >
                                {v.name}
                            </div>
                        ))}
                    </div>
                )}

                <button
                    onClick={(e) => {
                        e.stopPropagation();
                        setIsLiked((prev) => !prev);
                        toggleLike(name, liked);
                    }}
                    className='absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity'
                >
                    <FiHeart
                        className={cn(
                            'w-4 h-4',
                            isLiked ? 'text-red-500 fill-red-500' : 'text-white hover:text-red-400',
                        )}
                    />
                </button>
            </div>
            <div className='w-full px-4  items-center flex flex-row gap-2 mt-2'>
                <span className='text-lg'>{name}</span>
                <Button size='sm' variant='ghost' className='ml-auto text-zinc-400 hover:text-white hover:bg-red-700'>
                    <FiTrash className='w-4 h-4' />
                </Button>
            </div>
            <div className='flex flex-row gap-2 mt-2 w-full text-sm text-zinc-400 px-4 mb-3'>
                <span>{timestamp}</span>
                <span className='ml-auto'>{fileSize}</span>
            </div>
        </div>
    );
};

export const ClipPreview = memo(ClipPreviewComponent);

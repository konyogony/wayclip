import { ClipData } from '@/pages/all-clips';
import { revealItemInDir } from '@tauri-apps/plugin-opener';
import {
    Dialog,
    DialogBackdrop,
    DialogPanel,
    DialogTitle,
    DialogDescription,
    DialogHeader,
    DialogFooter,
} from '@/components/animate-ui/headless/dialog';
import { motion } from 'motion/react';
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuGroup,
    DropdownMenuItem,
    DropdownMenuLabel,
    DropdownMenuSeparator,
    DropdownMenuTrigger,
} from '@/components/animate-ui/radix/dropdown-menu';

import { cn } from '@/lib/utils';
import { memo, useMemo, useState, useCallback } from 'react';
import { FiPlay, FiHeart, FiMoreHorizontal, FiTrash, FiShare2 } from '@vertisanpro/react-icons/fi';
import { Button } from '@/components/ui/button';
import { invoke } from '@tauri-apps/api/core';

const convertLength = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const paddedSecs = secs.toString().padStart(2, '0');
    return `${mins}:${paddedSecs}`;
};

const convertTime = (ts: string): string => {
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

const ClipPreviewComponent = ({
    name,
    length,
    path,
    created_at,
    liked,
    size,
    tags,
    onDelete,
    updated_at,
}: ClipData & { onDelete: (path: string) => void }) => {
    const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
    const [isLiked, setIsLiked] = useState(liked);

    const duration = useMemo(() => {
        return convertLength(length);
    }, []);

    const created = useMemo(() => {
        return convertTime(created_at);
    }, []);

    const modified = useMemo(() => {
        return convertTime(updated_at);
    }, []);

    const fileSize = useMemo(() => {
        return convertSize(size);
    }, []);

    const handleDelete = useCallback(
        (path: string) => {
            onDelete(path);
            setIsDeleteDialogOpen(false);
            invoke('delete_clip', { pathStr: path }).catch((e) => {
                console.error(e);
            });
        },
        [onDelete],
    );

    const handleLike = useCallback((name: string, prev: boolean) => {
        setIsLiked((prev) => !prev);
        invoke('like_clip', { name: name, liked: !prev }).catch((e) => {
            console.error(e);
        });
    }, []);

    const handleOpenPath = useCallback(async (path: string) => {
        await revealItemInDir(path).catch((e) => console.error(e));
    }, []);

    return (
        <div className='group flex flex-col rounded-2xl w-full h-fit bg-[#18181b] overflow-clip gap-1 shadow-sm hover:border-zinc-500 transition-all duration-300 border-zinc-800 border'>
            <div className='relative aspect-video bg-zinc-800'>
                <div className='absolute inset-0 flex items-center justify-center'>
                    <div className='w-12 h-12 bg-zinc-700 rounded-full flex items-center justify-center opacity-60 group-hover:opacity-100 hover:scale-105 transition-opacity'>
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
                    onClick={() => handleLike(name, isLiked)}
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
                <Button size='sm' variant='ghost' className='ml-auto text-zinc-400 hover:text-white' asChild>
                    <motion.button whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }} className='w-fit'>
                        <FiShare2 className='w-4 h-4' />
                    </motion.button>
                </Button>

                <Button size='sm' variant='ghost' className='text-zinc-400 hover:text-white hover:bg-red-700' asChild>
                    <motion.button
                        whileHover={{ scale: 1.05 }}
                        whileTap={{ scale: 0.95 }}
                        className='w-fit'
                        onClick={() => setIsDeleteDialogOpen(true)}
                    >
                        <FiTrash className='w-4 h-4' />
                    </motion.button>
                </Button>

                <Dialog open={isDeleteDialogOpen} onClose={() => setIsDeleteDialogOpen(false)}>
                    <DialogBackdrop />

                    <DialogPanel className='sm:max-w-[425px]'>
                        <DialogHeader>
                            <DialogTitle>Delete clip?</DialogTitle>
                            <DialogDescription>
                                The clip will be permamently deleted and cannot be restored.
                            </DialogDescription>
                        </DialogHeader>
                        <DialogFooter>
                            <Button variant='outline' onClick={() => setIsDeleteDialogOpen(false)}>
                                Cancel
                            </Button>
                            <Button variant='destructive' type='submit' onClick={() => handleDelete(path)}>
                                Delete
                            </Button>
                        </DialogFooter>
                    </DialogPanel>
                </Dialog>

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
                    <DropdownMenuContent align='start' side='bottom' className='w-56'>
                        <DropdownMenuLabel>Actions</DropdownMenuLabel>
                        <DropdownMenuGroup>
                            <DropdownMenuItem>Rename</DropdownMenuItem>
                            <DropdownMenuItem>View</DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleLike(name, isLiked)}>Like</DropdownMenuItem>
                            <DropdownMenuItem>Share</DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleOpenPath(path)}>Open folder</DropdownMenuItem>
                            <DropdownMenuItem variant='destructive' onClick={() => setIsDeleteDialogOpen(true)}>
                                Delete
                            </DropdownMenuItem>
                        </DropdownMenuGroup>
                        <DropdownMenuSeparator />
                        <DropdownMenuGroup>
                            <DropdownMenuLabel>Info</DropdownMenuLabel>
                            <DropdownMenuItem disabled>Size: {fileSize}</DropdownMenuItem>
                            <DropdownMenuItem disabled>Duration: {duration}</DropdownMenuItem>
                            <DropdownMenuItem disabled>Created: {created}</DropdownMenuItem>
                            <DropdownMenuItem disabled>Modified: {modified}</DropdownMenuItem>
                            <DropdownMenuItem disabled className='truncate'>
                                Path: {path}
                            </DropdownMenuItem>
                        </DropdownMenuGroup>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
            <div className='flex flex-row gap-2 mt-2 w-full text-sm text-zinc-400 px-4 mb-3'>
                <span>{created}</span>
                <span className='ml-auto'>{fileSize}</span>
            </div>
        </div>
    );
};

export const ClipPreview = memo(ClipPreviewComponent);

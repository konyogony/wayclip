import { ClipData } from '@/pages/all-clips';
import { getPreview } from '@/lib/lib';
import { Skeleton } from '@/components/ui/skeleton';
import { Input } from '@/components/ui/input';
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
import { useLazyLoad } from '@/hooks/lazy';
import { convertSize, convertTime, convertLength, convertName } from '@/lib/lib';
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
import { memo, useMemo, useState, useCallback, useRef, useEffect } from 'react';
import { FiPlay, FiHeart, FiMoreHorizontal, FiTrash, FiShare2 } from '@vertisanpro/react-icons/fi';
import { Button } from '@/components/ui/button';
import { invoke } from '@tauri-apps/api/core';
import { NavLink } from 'react-router';

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
    const [clipName, setClipName] = useState(convertName(name, 'storeToDisplay'));
    const [clipPath, setClipPath] = useState(path);
    const renameInputRef = useRef<HTMLInputElement>(null);

    const [src, setSrc] = useState<string | null>(null);
    const videoRef = useRef<HTMLVideoElement>(null);
    const [previewRef, isVisible] = useLazyLoad<HTMLDivElement>();

    const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
    const [isRenameDialogOpen, setIsRenameDialogOpen] = useState(false);
    const [isVideoLoaded, setIsVideoLoaded] = useState(false);

    const [isLiked, setIsLiked] = useState(liked);

    const duration = useMemo(() => {
        return convertLength(length);
    }, []);

    const created = useMemo(() => {
        return convertTime(created_at);
    }, []);

    const isNew = useMemo(() => {
        const createdDate = new Date(created_at);
        const now = new Date();
        const diffMs = now.getTime() - createdDate.getTime();
        return diffMs < 24 * 60 * 60 * 1000;
    }, [created_at]);

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

    const handleRename = useCallback((path: string, newName: string) => {
        const clean = convertName(newName, 'displayToStore');
        setIsRenameDialogOpen(false);
        const lastSlashIndex = path.lastIndexOf('/');
        const dir = lastSlashIndex >= 0 ? path.slice(0, lastSlashIndex + 1) : '';
        const oldFileName = lastSlashIndex >= 0 ? path.slice(lastSlashIndex + 1) : path;
        const extMatch = oldFileName.match(/\.[^/.]+$/);
        const ext = extMatch ? extMatch[0] : '';
        setClipPath(dir + clean + ext);
        setClipName(newName);
        invoke('rename_clip', { pathStr: path, newName: clean }).catch((e) => console.error(e));
    }, []);

    useEffect(() => {
        if (isVisible && !src) {
            getPreview(path)
                .then((src) => {
                    console.log(src);
                    setSrc(src);
                })
                .catch((e) => console.error(`Failed to load preview for ${path}: ${e}`));
        }

        return () => {
            if (src) {
                if (videoRef.current) {
                    videoRef.current.src = '';
                    videoRef.current.removeAttribute('src');
                    videoRef.current.load();
                }
            }
        };
    }, [isVisible, path, src]);

    const handleVideoError = (e: React.SyntheticEvent<HTMLVideoElement, Event>) => {
        console.error(`Video Error for ${path}:`, e.currentTarget.error);
    };

    return (
        <div
            ref={previewRef}
            className='group flex flex-col rounded-2xl w-full h-fit bg-[#18181b] overflow-clip gap-1 shadow-sm hover:border-zinc-500 transition-all duration-300 border-zinc-800 border'
        >
            <div className='relative aspect-video bg-zinc-800 rounded-none'>
                {!isVideoLoaded && <Skeleton className='absolute inset-0 h-full w-full z-10' />}
                {src && (
                    <video
                        ref={videoRef}
                        src={src}
                        muted
                        loop
                        onError={handleVideoError}
                        className={cn('w-full h-full transition-opacity', isVideoLoaded ? 'opacity-100' : 'opacity-0')}
                        onLoadedData={() => setIsVideoLoaded(true)}
                        onMouseEnter={() => videoRef.current?.play().catch(() => {})}
                        onMouseLeave={() => {
                            if (videoRef.current) {
                                videoRef.current.pause();
                                videoRef.current.currentTime = 0;
                            }
                        }}
                    />
                )}

                {isNew && <div className='rounded-2xl bg-red-500 px-2 text-xs absolute top-2 left-2 z-10'>New!</div>}

                <NavLink
                    to={`/video/${convertName(clipName, 'displayToStore')}`}
                    className='absolute inset-0 flex items-center justify-center z-20'
                >
                    <div className='w-12 h-12 bg-zinc-700 rounded-full flex items-center justify-center opacity-60 group-hover:opacity-100 hover:scale-105 transition-opacity'>
                        <FiPlay className='w-5 h-5 text-white ml-0.5' />
                    </div>
                </NavLink>

                <div className='absolute bottom-2 right-2 bg-black/80 text-white text-xs px-2 py-1 rounded z-20'>
                    {duration}
                </div>

                {tags && (
                    <div className='absolute bottom-2 left-2 flex flex-row gap-1 text-white text-xs z-20'>
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
                    className='absolute top-3 right-3 opacity-0 group-hover:opacity-100 transition-opacity z-20'
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
                <span className='text-lg'>{clipName}</span>
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
                            <Button variant='destructive' type='submit' onClick={() => handleDelete(clipPath)}>
                                Delete
                            </Button>
                        </DialogFooter>
                    </DialogPanel>
                </Dialog>

                <Dialog open={isRenameDialogOpen} onClose={() => setIsRenameDialogOpen(false)}>
                    <DialogBackdrop />

                    <DialogPanel className='sm:max-w-[425px]'>
                        <DialogHeader>
                            <DialogTitle>Rename clip</DialogTitle>
                            <DialogDescription>The name that will be shown in the Wayclip App.</DialogDescription>
                        </DialogHeader>
                        <div className='mt-4'>
                            <Input ref={renameInputRef} defaultValue={clipName} placeholder='Enter new clip name' />
                        </div>
                        <DialogFooter>
                            <Button variant='outline' onClick={() => setIsRenameDialogOpen(false)}>
                                Cancel
                            </Button>
                            <Button
                                variant='default'
                                type='submit'
                                onClick={() => {
                                    const value = renameInputRef.current?.value || '';
                                    handleRename(clipPath, value);
                                }}
                            >
                                Confirm
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
                    <DropdownMenuContent align='start' side='bottom' className='w-64'>
                        <DropdownMenuLabel>Actions</DropdownMenuLabel>
                        <DropdownMenuGroup>
                            <DropdownMenuItem onClick={() => setIsRenameDialogOpen(true)}>Rename</DropdownMenuItem>
                            <DropdownMenuItem asChild>
                                <NavLink to={`/video/${convertName(clipName, 'displayToStore')}`}>View</NavLink>
                            </DropdownMenuItem>
                            <DropdownMenuItem
                                onClick={() => handleLike(convertName(clipName, 'displayToStore'), isLiked)}
                            >
                                Like
                            </DropdownMenuItem>
                            <DropdownMenuItem>Share</DropdownMenuItem>
                            <DropdownMenuItem onClick={() => handleOpenPath(clipPath)}>Open folder</DropdownMenuItem>
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
                            {/* <DropdownMenuItem disabled className='truncate'>
                                <SmartTickerDraggable
                                    smart={false}
                                    isText={true}
                                    autoFill={false}
                                    playOnHover={true}
                                    direction={'left'}
                                    rtl={false}
                                    infiniteScrollView={false}
                                    speed={60}
                                    delay={0}
                                    delayBack={0}
                                    iterations={'infinite'}
                                    disableSelect={false}
                                >
                                    Path: {clipPath.replace(/^\/home\/[^/]+\//, '~/')}
                                </SmartTickerDraggable>
                            </DropdownMenuItem>
                            */}
                        </DropdownMenuGroup>
                    </DropdownMenuContent>
                </DropdownMenu>
            </div>
            <div className='flex flex-row gap-2 mt-2 w-full text-sm text-zinc-400 px-4 mb-3'>
                <span>{created}</span>
                <span className='ml-auto font-mono'>{fileSize}</span>
            </div>
        </div>
    );
};

export const ClipPreview = memo(ClipPreviewComponent);

import { useEffect, useState, useCallback, useRef } from 'react';
import { useDebounce } from '@/hooks/debounce';
import { cn } from '@/lib/utils';
import { useSidebar } from '@/hooks/sidebar';
import {
    Pagination,
    PaginationContent,
    PaginationEllipsis,
    PaginationItem,
    PaginationLink,
    PaginationNext,
    PaginationPrevious,
} from '@/components/ui/pagination';
import { invoke } from '@tauri-apps/api/core';
import { ClipPreview } from '@/components/clip-preview';
import { Input } from '@/components/ui/input';
import { Button } from '@/components/ui/button';
import { FaMagnifyingGlass, FaSort, FaXmark } from '@vertisanpro/react-icons/fa6';
import { FiFilter, FiSidebar } from '@vertisanpro/react-icons/fi';
import { VscLoading } from '@vertisanpro/react-icons/vsc';
import { useQuery, useMutation, useQueryClient, keepPreviousData } from '@tanstack/react-query';

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

export interface PaginatedClips {
    clips: ClipData[];
    total_pages: number;
    total_clips: number;
}

const CLIPS_PER_PAGE = 16;

const fetchClips = async (page: number, searchQuery: string): Promise<PaginatedClips> => {
    return await invoke<PaginatedClips>('pull_clips', {
        page: page,
        pageSize: CLIPS_PER_PAGE,
        searchQuery: searchQuery || null,
    });
};

const AllClips = () => {
    const [currentPage, setCurrentPage] = useState(1);
    const [searchQuery, setSearchQuery] = useState('');
    const debouncedSearchQuery = useDebounce(searchQuery, 300);

    const { toggleSidebar } = useSidebar();
    const queryClient = useQueryClient();

    const isInitialSearch = useRef(true);

    const { isLoading, data } = useQuery<PaginatedClips, Error>({
        queryKey: ['clips', currentPage, debouncedSearchQuery],
        queryFn: () => fetchClips(currentPage, debouncedSearchQuery),
        placeholderData: keepPreviousData,
    });

    const clips = data?.clips ?? [];
    const totalPages = data?.total_pages ?? 0;
    const totalClips = data?.total_clips ?? 0;

    useEffect(() => {
        if (isInitialSearch.current) {
            isInitialSearch.current = false;
            return;
        }
        setCurrentPage(1);
    }, [debouncedSearchQuery]);

    const deleteClipMutation = useMutation({
        mutationFn: (path: string) => invoke('delete_clip', { pathStr: path }),
        onMutate: async (deletedPath: string) => {
            await queryClient.cancelQueries({ queryKey: ['clips'] });
            const queryKey = ['clips', currentPage, debouncedSearchQuery];
            const previousClips = queryClient.getQueryData<PaginatedClips>(queryKey);
            if (previousClips) {
                queryClient.setQueryData<PaginatedClips>(queryKey, {
                    ...previousClips,
                    clips: previousClips.clips.filter((clip) => clip.path !== deletedPath),
                    total_clips: previousClips.total_clips - 1,
                });
            }
            return { previousClips, queryKey };
        },

        onError: (err, _deletedPath, context) => {
            console.error('Optimistic delete failed, rolling back:', err);
            if (context?.previousClips) {
                queryClient.setQueryData(context.queryKey, context.previousClips);
            }
        },

        onSettled: (_data, _error, _variables, context) => {
            queryClient.invalidateQueries({ queryKey: context?.queryKey });
        },
    });

    const handleDelete = useCallback(
        (path: string) => {
            deleteClipMutation.mutate(path);
        },
        [deleteClipMutation],
    );

    const handlePageChange = useCallback(
        (page: number) => {
            if (page >= 1 && page <= totalPages) {
                setCurrentPage(page);
            }
        },
        [totalPages],
    );

    const renderPaginationLinks = () => {
        const pageNumbers = [];
        const siblingCount = 1;

        if (totalPages <= 1) return null;

        pageNumbers.push(
            <PaginationItem key={1}>
                <PaginationLink onClick={() => handlePageChange(1)} isActive={currentPage === 1}>
                    1
                </PaginationLink>
            </PaginationItem>,
        );

        const leftSiblingIndex = Math.max(currentPage - siblingCount, 2);
        const rightSiblingIndex = Math.min(currentPage + siblingCount, totalPages - 1);

        if (leftSiblingIndex > 2) {
            pageNumbers.push(
                <PaginationItem key='left-ellipsis'>
                    <PaginationEllipsis />
                </PaginationItem>,
            );
        }

        for (let i = leftSiblingIndex; i <= rightSiblingIndex; i++) {
            pageNumbers.push(
                <PaginationItem key={i}>
                    <PaginationLink onClick={() => handlePageChange(i)} isActive={currentPage === i}>
                        {i}
                    </PaginationLink>
                </PaginationItem>,
            );
        }

        if (rightSiblingIndex < totalPages - 1) {
            pageNumbers.push(
                <PaginationItem key='right-ellipsis'>
                    <PaginationEllipsis />
                </PaginationItem>,
            );
        }

        if (totalPages > 1 && currentPage !== totalPages) {
            pageNumbers.push(
                <PaginationItem key={totalPages}>
                    <PaginationLink onClick={() => handlePageChange(totalPages)}>{totalPages}</PaginationLink>
                </PaginationItem>,
            );
        }
        return pageNumbers;
    };

    const renderContent = () => {
        if (isLoading && !data) {
            return (
                <div className='flex flex-col mx-auto my-auto items-center gap-4'>
                    <VscLoading className='w-8 h-8 animate-spin' />
                    <span className='text-lg text-zinc-300'>Loading clips...</span>
                </div>
            );
        }

        if (clips.length === 0) {
            if (debouncedSearchQuery) {
                return (
                    <div className='flex flex-col mx-auto my-auto items-center'>
                        <span className='text-lg'>No clips match your search</span>
                        <span>Try a different search query or clear the search field.</span>
                    </div>
                );
            }
            return (
                <div className='flex flex-col mx-auto my-auto items-center'>
                    <span className='text-lg'>No clips found</span>
                    <span>Seems like you dont have any clips saved. They will appear here.</span>
                </div>
            );
        }

        return (
            <div className={cn('flex-grow overflow-y-auto p-8')}>
                {totalPages > 1 && (
                    <div className='mb-8 flex justify-center'>
                        <Pagination>
                            <PaginationContent>
                                <PaginationItem>
                                    <PaginationPrevious
                                        className={cn({
                                            'pointer-events-none text-zinc-600': currentPage === 1,
                                            'cursor-pointer': currentPage !== 1,
                                        })}
                                        onClick={() => handlePageChange(currentPage - 1)}
                                    />
                                </PaginationItem>
                                {renderPaginationLinks()}
                                <PaginationItem>
                                    <PaginationNext
                                        className={cn({
                                            'pointer-events-none text-zinc-600': currentPage === totalPages,
                                            'cursor-pointer': currentPage !== totalPages,
                                        })}
                                        onClick={() => handlePageChange(currentPage + 1)}
                                    />
                                </PaginationItem>
                            </PaginationContent>
                        </Pagination>
                    </div>
                )}

                <div className='w-full h-fit grid-cols-1 md:grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4 grid gap-6'>
                    {clips.map((clip) => (
                        <ClipPreview {...clip} key={clip.path} onDelete={handleDelete} />
                    ))}
                </div>

                {totalPages > 1 && (
                    <div className='mt-8 flex justify-center'>
                        <Pagination>
                            <PaginationContent>
                                <PaginationItem>
                                    <PaginationPrevious
                                        className={cn({
                                            'pointer-events-none text-zinc-600': currentPage === 1,
                                            'cursor-pointer': currentPage !== 1,
                                        })}
                                        onClick={() => handlePageChange(currentPage - 1)}
                                    />
                                </PaginationItem>
                                {renderPaginationLinks()}
                                <PaginationItem>
                                    <PaginationNext
                                        className={cn({
                                            'pointer-events-none text-zinc-600': currentPage === totalPages,
                                            'cursor-pointer': currentPage !== totalPages,
                                        })}
                                        onClick={() => handlePageChange(currentPage + 1)}
                                    />
                                </PaginationItem>
                            </PaginationContent>
                        </Pagination>
                    </div>
                )}
            </div>
        );
    };

    return (
        <div className='flex flex-col w-full h-full bg-zinc-950 text-white'>
            <div className='flex items-center gap-3 w-full border-b border-zinc-800 py-4 px-10 flex-shrink-0'>
                <button
                    className='flex flex-row items-center cursor-pointer gap-2 w-fit h-fit justify-center transition-all duration-200 ease-in-out rounded-lg p-2 z-30 hover:bg-zinc-800/50'
                    onClick={toggleSidebar}
                >
                    <FiSidebar size={18} />
                </button>
                <div className='w-[1px] h-8 mr-1 bg-zinc-800' />
                <h1 className='text-2xl font-bold'>All Clips</h1>
                {!isLoading && totalClips > 0 && <span className='text-zinc-300 text-sm'>{totalClips} clips </span>}
                <div className='relative ml-auto'>
                    <FaMagnifyingGlass className='absolute left-3 top-1/2 transform -translate-y-1/2 text-zinc-400 w-4 h-4' />
                    <Input
                        placeholder='Search...'
                        value={searchQuery}
                        onChange={(e) => setSearchQuery(e.target.value)}
                        className='pl-10 w-80 bg-zinc-900 border-zinc-700 text-white placeholder:text-zinc-400 focus:border-zinc-600'
                    />
                    {searchQuery && (
                        <FaXmark
                            className='absolute right-3 top-1/2 -translate-y-1/2 text-zinc-400 size-4 z-10 cursor-pointer hover:text-white'
                            onClick={() => setSearchQuery('')}
                        />
                    )}
                </div>
                <Button variant='ghost' size='icon' className='text-zinc-400 hover:text-white hover:bg-zinc-800'>
                    <FiFilter className='w-4 h-4' />
                </Button>
                <Button variant='ghost' size='icon' className='text-zinc-400 hover:text-white hover:bg-zinc-800'>
                    <FaSort className='w-4 h-4' />
                </Button>
            </div>

            {renderContent()}
        </div>
    );
};

export default AllClips;

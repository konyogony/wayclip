const Editor = () => {
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
        </div>
    );
};

export default Editor;

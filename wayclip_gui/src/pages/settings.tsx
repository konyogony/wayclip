const Settings = () => {
    return (
        <div className='w-full h-full flex flex-col px-8 py-4'>
            <span className='text-2xl text-zinc-200 mt-4 mb-8 font-semibold'>Settings</span>
            <div className='flex flex-col gap-4'>
                <span className='text-sm text-zinc-400'>General</span>
                <div className='flex flex-flex items-center gap-2'>
                    <span className='text-zinc-200'>Stuff</span>
                </div>
            </div>
        </div>
    );
};

export default Settings;

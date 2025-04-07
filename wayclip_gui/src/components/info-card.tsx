import { BsGithub } from '@vertisanpro/react-icons/bs';
import packageJson from '../../package.json';

export const InfoCard = () => {
    return (
        <div className='w-full flex flex-col mt-auto text-sm justify-center gap-2 items-center rounded-lg px-8 py-6 min-h-1/8 bg-indigo-300/10'>
            <span className='text-lg text-zinc-200 font-semibold'>Wayclip v{packageJson.version}</span>
            <span className='text-zinc-400 text-center flex flex-wrap gap-1'>
                An open-source clipping tool for wayland, built in rust, tauri and react. Support this project by giving
                it a star on github!
            </span>
            <div className='flex flex-row gap-2'>
                <a href='https://github.com/konyogony/wayclip' target='_blank' rel='noopener noreferrer'>
                    <BsGithub
                        size={32}
                        className='text-zinc-200 hover:text-zinc-300 transition-all duration-200 ease-in-out'
                    />
                </a>
            </div>
        </div>
    );
};

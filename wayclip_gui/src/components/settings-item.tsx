import { useState } from 'react';
import { Setting } from '@/lib/types';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/tooltip';

interface SettingsItemProps extends Setting {
    handleSave: (key: string, value: string | number | boolean) => void;
}

export const SettingsItem = ({
    name,
    description,
    tooltip,
    type,
    defaultValue,
    storageKey,
    handleSave,
}: SettingsItemProps) => {
    const [value, setValue] = useState(defaultValue);
    return (
        <div className='flex flex-flex items-center gap-2'>
            <div className='flex flex-col gap-1 w-1/3'>
                <span className='text-zinc-200'>{name}</span>
                {!tooltip ? (
                    <span className='text-zinc-400 text-sm'>{description}</span>
                ) : (
                    <TooltipProvider>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <span className='text-zinc-400 text-sm'>{description}</span>
                            </TooltipTrigger>
                            <TooltipContent>
                                <span className='text-zinc-200 text-sm'>{tooltip}</span>
                            </TooltipContent>
                        </Tooltip>
                    </TooltipProvider>
                )}
            </div>
            {type === 'boolean' && (
                <input
                    type='checkbox'
                    className='w-4 h-4 bg-zinc-900/50 text-indigo-500 rounded-lg'
                    checked={value as boolean}
                    onChange={(e) => setValue(e.target.checked)}
                />
            )}
            {type === 'select' && (
                <select
                    className='bg-zinc-900/50 w-1/2 text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400 focus:ring-0 focus:outline-0'
                    value={value as string}
                    onChange={(e) => setValue(e.target.value)}
                >
                    <option value='option1'>Option 1</option>
                    <option value='option2'>Option 2</option>
                </select>
            )}
            {type === 'number' && (
                <input
                    type='number'
                    className='bg-zinc-900/50 w-1/2 text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400 focus:ring-0 focus:outline-0'
                    value={value as number}
                    onChange={(e) => setValue(Number(e.target.value))}
                />
            )}
            {type === 'string' && (
                <input
                    type='text'
                    value={value as string}
                    onChange={(e) => setValue(e.target.value)}
                    className='bg-zinc-900/50 w-1/2 text-zinc-300 focus:text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400 focus:ring-0 focus:outline-0'
                    placeholder='Enter format...'
                />
            )}
            <button
                onClick={() => handleSave(storageKey, value)}
                className='bg-indigo-500 hover:bg-indigo-600 transition-all duration-200 ease-in-out rounded-lg px-4 text-sm py-2 text-zinc-200'
            >
                Save
            </button>
        </div>
    );
};

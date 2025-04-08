import { useState, useEffect } from 'react';
import { FiChevronDown, FiHelpCircle, FiCheck } from '@vertisanpro/react-icons/fi';
import { JsonValue, Setting } from '@/lib/types';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/tooltip';

interface SettingsItemProps extends Setting {
    handleSave: (key: string, value: JsonValue) => void;
}

export const SettingsItem = ({
    name,
    description,
    tooltip,
    type,
    storageKey,
    defaultValue,
    currentValue,
    options,
    handleSave,
}: SettingsItemProps) => {
    const [value, setValue] = useState(currentValue as JsonValue);

    useEffect(() => {
        setValue(currentValue as JsonValue);
    }, [currentValue]);

    const handleChange = (newValue: JsonValue) => {
        setValue(newValue);
    };

    const handleReset = () => {
        setValue(defaultValue as JsonValue);
        handleSave(storageKey, defaultValue as JsonValue);
    };

    return (
        <div className='flex items-center flex-row gap-2'>
            <div className='flex flex-col gap-1 w-1/3'>
                <span className='text-zinc-200'>{name}</span>
                {!tooltip ? (
                    <span className='text-zinc-400 text-sm'>{description}</span>
                ) : (
                    <TooltipProvider>
                        <Tooltip>
                            <TooltipTrigger asChild>
                                <span className='text-zinc-400 text-sm flex flex-wrap gap-1 items-center'>
                                    {description} <FiHelpCircle size={14} />
                                </span>
                            </TooltipTrigger>
                            <TooltipContent>
                                <span className='text-zinc-200 text-sm'>{tooltip}</span>
                            </TooltipContent>
                        </Tooltip>
                    </TooltipProvider>
                )}
            </div>

            {type === 'boolean' && (
                <div className='flex items-center  w-1/2'>
                    <input
                        type='checkbox'
                        checked={value as boolean}
                        onChange={(e) => handleChange(e.target.checked)}
                        className='appearance-none size-5 bg-zinc-900/50 rounded-sm border-2 border-zinc-700 checked:bg-indigo-500 focus:ring-0 focus:outline-0'
                    />
                    {value === true && <FiCheck className='absolute pointer-events-none' size={20} />}
                </div>
            )}

            {type === 'select' && (
                <div className='relative w-1/2'>
                    <select
                        value={value as string}
                        onChange={(e) => {
                            const val = e.target.value;
                            if (!isNaN(Number(val))) handleChange(Number(val));
                            else handleChange(val);
                        }}
                        className='bg-zinc-900/50 relative w-full text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400 focus:ring-0 focus:outline-0 appearance-none'
                    >
                        {options?.map((option, index) => (
                            <option key={index} value={option}>
                                {option}
                            </option>
                        ))}
                    </select>
                    <FiChevronDown
                        size={22}
                        className='absolute right-2 top-1/2 z-20 -translate-y-1/2 text-zinc-400 pointer-events-none'
                    />
                </div>
            )}

            {type === 'number' && (
                <input
                    type='number'
                    value={value as number}
                    onChange={(e) => handleChange(Number(e.target.value))}
                    className='bg-zinc-900/50 w-1/2 text-zinc-200 rounded-lg px-4 py-2 placeholder:text-zinc-400 focus:ring-0 focus:outline-0'
                />
            )}

            {type === 'string' && (
                <input
                    type='text'
                    value={value as string}
                    onChange={(e) => handleChange(e.target.value)}
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

            <button
                onClick={handleReset}
                className='bg-red-500 hover:bg-red-600 transition-all duration-200 ease-in-out rounded-lg px-4 text-sm py-2 text-zinc-200'
            >
                Reset
            </button>
        </div>
    );
};

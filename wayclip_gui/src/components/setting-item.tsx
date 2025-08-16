import type { Setting, JsonValue } from '@/lib/types';
import { Slider, SliderValue } from '@/components/ui/slider';
import { cn } from '@/lib/utils';
import { categories } from '@/lib/types';
import { Input } from '@/components/ui/input';
import { Switch } from '@/components/animate-ui/base/switch';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Info } from 'lucide-react';

interface SettingsItemProps extends Setting {
    value: JsonValue;
    onChange: (key: string, value: JsonValue) => void;
}

export const SettingsItem = ({
    name,
    description,
    tooltip,
    type,
    options,
    defaultValue,
    storageKey,
    value,
    category,
    onChange,
}: SettingsItemProps) => {
    const renderInput = () => {
        switch (type) {
            case 'boolean':
                return (
                    <div className='flex items-center space-x-3'>
                        <Switch
                            checked={value as boolean}
                            onCheckedChange={(checked) => onChange(storageKey, checked)}
                            className='data-[state=checked]:bg-blue-600'
                        />
                        <span className='text-sm text-zinc-400'>{value ? 'Enabled' : 'Disabled'}</span>
                    </div>
                );

            case 'select':
                const normalizedOptions = options?.map((opt) =>
                    typeof opt === 'string' || typeof opt === 'number'
                        ? { label: String(opt), value: String(opt) }
                        : { label: opt.name, value: opt.node_name },
                );
                return (
                    <Select value={String(value)} onValueChange={(newValue) => onChange(storageKey, newValue)}>
                        <SelectTrigger
                            className={cn(
                                'bg-zinc-800 border-zinc-700 text-white focus:border-zinc-600',
                                category === categories.audio ? 'w-64' : 'w-48',
                            )}
                        >
                            <SelectValue placeholder='Select a device...' />
                        </SelectTrigger>
                        <SelectContent className='bg-zinc-800 border-zinc-700'>
                            {normalizedOptions?.map((option) => (
                                <SelectItem
                                    key={option.value}
                                    value={option.value}
                                    className='text-white hover:bg-zinc-700 focus:bg-zinc-700'
                                >
                                    {option.label}
                                </SelectItem>
                            ))}
                        </SelectContent>
                    </Select>
                );

            case 'string':
            case 'number':
                return (
                    <Input
                        type={type === 'number' ? 'number' : 'text'}
                        value={String(value)}
                        onChange={(e) => {
                            const newValue = type === 'number' ? Number(e.target.value) : e.target.value;
                            onChange(storageKey, newValue);
                        }}
                        className='w-64 bg-zinc-800 border-zinc-700 text-white placeholder:text-zinc-400 focus:border-zinc-600'
                        placeholder={String(defaultValue)}
                    />
                );

            case 'slider':
                return (
                    <Slider
                        className='w-48'
                        defaultValue={Number(defaultValue)}
                        onValueChange={(val) => {
                            if (typeof val === 'number') {
                                onChange(storageKey, val);
                            }
                        }}
                        min={0}
                        max={100}
                        step={1}
                    >
                        <div className='flex justify-between'>
                            <SliderValue />
                        </div>
                    </Slider>
                );

            default:
                return null;
        }
    };

    return (
        <div className='flex items-center justify-between py-4 border-b border-zinc-800 last:border-b-0'>
            <div className='flex-1 space-y-1'>
                <div className='flex items-center gap-2'>
                    <h3 className='font-medium text-white'>{name}</h3>
                    {tooltip && (
                        <div className='group relative'>
                            <Info className='w-4 h-4 text-zinc-400 cursor-help' />
                            <div className='absolute bottom-full left-1/2 transform -translate-x-1/2 mb-2 px-3 py-2 bg-zinc-800 text-white text-xs rounded-lg opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none whitespace-nowrap z-10 border border-zinc-700'>
                                {tooltip}
                            </div>
                        </div>
                    )}
                </div>
                <p className='text-sm text-zinc-400 max-w-md'>{description}</p>
            </div>
            <div className='ml-4'>{renderInput()}</div>
        </div>
    );
};

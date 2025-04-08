import { invoke } from '@tauri-apps/api/core';
import { JsonObject, JsonValue, Setting, categories } from '@/lib/types';
import { SettingsItem } from '@/components/settings-item';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { defaultSettings } from '@/lib/config';

const handleSave = async (key: string, value: JsonValue) => {
    if (
        !defaultSettings.map((setting) => setting.storageKey).includes(key) ||
        typeof value === 'undefined' ||
        value === null ||
        value === ''
    ) {
        console.log(key, value);
        console.error('[TS]: Invalid storage key:', key, 'or value:', value);
        toast.error('[TS]: Unable to save settings. Either key does exist, or value is invalid.');
        return;
    }
    try {
        await invoke('update_settings', { key, value });
        toast.success('Setting saved successfully.');
    } catch (e: any) {
        console.error('Error saving settings:', e);
        toast.error(e as string);
    }
};

const Settings = () => {
    const [currentSettings, setCurrentSettings] = useState<Setting[]>(defaultSettings);

    useEffect(() => {
        const pullSettings = async () => {
            const res: JsonObject = await invoke('pull_settings');
            setCurrentSettings((prevSettings) => {
                return prevSettings.map((setting) => {
                    const updatedValue = res[setting.storageKey];
                    if (updatedValue === undefined || updatedValue === null) {
                        console.error(`Unable to pull settings for ${setting.storageKey}. updatedValue:`, updatedValue);
                        toast.error(`Unable to pull settings for ${setting.storageKey}. Please try again.`);
                        return setting;
                    }
                    return {
                        ...setting,
                        currentValue: updatedValue,
                    };
                });
            });
        };

        pullSettings();
    }, []);

    return (
        <div className='w-full h-full flex flex-col px-8 py-4'>
            <span className='text-2xl text-zinc-200 mt-4 mb-8 font-semibold'>Settings</span>
            {Object.values(categories).map((v, i) => (
                <div key={i} className='flex flex-col gap-6 mb-16'>
                    <span className='text-sm text-zinc-400 -mb-2'>{v}</span>
                    {currentSettings
                        .filter((setting) => setting.category === v)
                        .map((setting, index) => {
                            return <SettingsItem key={index} {...setting} handleSave={handleSave} />;
                        })}
                </div>
            ))}
        </div>
    );
};

export default Settings;

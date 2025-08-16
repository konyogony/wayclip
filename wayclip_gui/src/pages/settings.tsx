import { invoke } from '@tauri-apps/api/core';
import { cn } from '@/lib/utils';
import { useSidebar } from '@/hooks/sidebar';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { JsonObject, JsonValue, Setting, categories, AudioDevice } from '@/lib/types';
import { SettingsItem } from '@/components/setting-item';
import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { defaultSettings } from '@/lib/config';
import { FiRefreshCw, FiSave, FiSidebar } from '@vertisanpro/react-icons/fi';

const showToast = (message: string, type: 'success' | 'error' = 'success') => {
    if (type === 'success') {
        toast.success(message);
    } else {
        toast.error(message);
    }
    console.log(`Toast (${type}): ${message}`);
};

const Settings = () => {
    const [currentSettings, setCurrentSettings] = useState<Setting[]>(defaultSettings);
    const [pendingChanges, setPendingChanges] = useState<Record<string, JsonValue>>({});
    const [audioDevices, setAudioDevices] = useState<AudioDevice[]>([]);
    const [isRefreshing, setIsRefreshing] = useState(false);
    const [savingCategories, setSavingCategories] = useState<Set<string>>(new Set());

    const { toggleSidebar } = useSidebar();

    const handleChange = (key: string, value: JsonValue) => {
        const setting = currentSettings.find((s) => s.storageKey === key);

        if (setting && typeof setting.defaultValue === 'number') {
            const numericValue = Number(value);
            setPendingChanges((prev) => ({
                ...prev,
                [key]: isNaN(numericValue) ? getSettingValue(setting) : numericValue,
            }));
        } else {
            setPendingChanges((prev) => ({ ...prev, [key]: value }));
        }
    };

    const saveCategory = async (categoryName: string) => {
        const categorySettings = currentSettings.filter((s) => s.category === categoryName);
        const categoryChanges = Object.entries(pendingChanges).filter(([key]) =>
            categorySettings.some((s) => s.storageKey === key),
        );

        if (categoryChanges.length === 0) return;

        setSavingCategories((prev) => new Set(prev).add(categoryName));

        try {
            for (const [key, value] of categoryChanges) {
                await invoke('update_settings', { key, value });
            }

            setCurrentSettings((prev) =>
                prev.map((setting) => {
                    const newValue = pendingChanges[setting.storageKey];
                    return newValue !== undefined ? { ...setting, currentValue: newValue } : setting;
                }),
            );

            setPendingChanges((prev) => {
                const updated = { ...prev };
                categoryChanges.forEach(([key]) => delete updated[key]);
                return updated;
            });

            showToast(`${categoryName} settings saved successfully.`);
        } catch (error) {
            console.error('Error saving settings:', error);
            showToast(`Failed to save ${categoryName} settings.`, 'error');
        } finally {
            setSavingCategories((prev) => {
                const updated = new Set(prev);
                updated.delete(categoryName);
                return updated;
            });
        }
    };

    const refreshAudioDevices = async () => {
        setIsRefreshing(true);
        try {
            const devices = await invoke<AudioDevice[]>('get_all_audio_devices_command');
            setAudioDevices(devices);
            console.log(devices);
        } catch (error) {
            console.error('Failed to refresh audio devices:', error);
            showToast('Failed to refresh audio devices.', 'error');
        } finally {
            setIsRefreshing(false);
        }
    };

    const getCategoryChanges = (categoryName: string) => {
        const categorySettings = currentSettings.filter((s) => s.category === categoryName);
        return Object.entries(pendingChanges).filter(([key]) => categorySettings.some((s) => s.storageKey === key))
            .length;
    };

    const getSettingValue = (setting: Setting) => {
        const rawValue = pendingChanges[setting.storageKey] ?? setting.currentValue ?? setting.defaultValue;

        if (setting.category === categories.audio && setting.type === 'select') {
            const isValidDevice = audioDevices.some((device) => device.node_name === rawValue);
            return isValidDevice ? rawValue : '';
        }

        return rawValue;
    };

    useEffect(() => {
        const pullSettings = async () => {
            try {
                await refreshAudioDevices();
                const fetchedValues: JsonObject = await invoke('pull_settings');
                const newSettings = defaultSettings.map((setting) => {
                    const pulledValue = fetchedValues[setting.storageKey];
                    return {
                        ...setting,
                        currentValue: pulledValue ?? setting.defaultValue,
                    };
                });
                setCurrentSettings(newSettings);
            } catch (error) {
                console.error('Failed to pull settings:', error);
                showToast('Failed to load settings.', 'error');
            } finally {
                showToast('Settings loaded.');
            }
        };

        pullSettings();
    }, []);

    return (
        <div className='gap-y-8 w-full pb-8 flex flex-col'>
            <div className='flex items-center gap-3 w-full border-b border-zinc-800 py-4 px-10 flex-shrink-0'>
                <button
                    className={cn(
                        'flex flex-row items-center cursor-pointer gap-2 w-fit h-fit justify-center transition-all duration-200 ease-in-out rounded-lg p-2 z-30 hover:bg-zinc-800/50',
                    )}
                    onClick={toggleSidebar}
                >
                    <FiSidebar size={18} />
                </button>
                <div className='w-[1px] h-8 mr-1 bg-zinc-800' />
                <h1 className='text-2xl font-bold'>Settings</h1>
            </div>

            {Object.values(categories).map((categoryName) => {
                const hasChanges = getCategoryChanges(categoryName) > 0;
                const isSaving = savingCategories.has(categoryName);
                const categorySettings = currentSettings.filter((setting) => setting.category === categoryName);

                return (
                    <div key={categoryName} className='gap-y-4 pt-8 w-2xl mx-auto'>
                        <div className='flex items-center justify-between'>
                            <div className='flex items-center gap-3'>
                                <h2 className='text-lg font-semibold text-white'>{categoryName}</h2>
                                <div className='h-px bg-zinc-800 flex-1' />
                            </div>

                            {hasChanges && (
                                <Button
                                    onClick={() => saveCategory(categoryName)}
                                    disabled={isSaving}
                                    size='sm'
                                    className='bg-blue-600 hover:bg-blue-700 text-white'
                                >
                                    <FiSave className='w-4 h-4 mr-2' />
                                    {isSaving ? 'Saving...' : 'Save Changes'}
                                </Button>
                            )}
                        </div>

                        <Card className='bg-zinc-900/50 border-zinc-800 mt-4'>
                            <CardContent className='px-6'>
                                <div className='gap-y-0'>
                                    {categorySettings.map((setting, index) => (
                                        <SettingsItem
                                            key={index}
                                            {...setting}
                                            options={
                                                setting.type === 'select' && setting.category === categories.audio
                                                    ? setting.storageKey === 'mic_node_name'
                                                        ? audioDevices.filter((d) =>
                                                              d.node_name.startsWith('alsa_input'),
                                                          )
                                                        : audioDevices.filter((d) =>
                                                              d.node_name.startsWith('alsa_output'),
                                                          )
                                                    : setting.options
                                            }
                                            value={getSettingValue(setting)}
                                            onChange={handleChange}
                                        />
                                    ))}
                                    {categoryName === categories.audio && (
                                        <div className='flex justify-end pt-4'>
                                            <Button
                                                onClick={() => {
                                                    refreshAudioDevices();
                                                    toast.success('Refreshed audio devices');
                                                }}
                                                disabled={isRefreshing}
                                                size='sm'
                                                variant='outline'
                                                className='border-zinc-700 hover:bg-zinc-800 bg-transparent text-zinc-300'
                                            >
                                                <FiRefreshCw
                                                    className={cn('w-4 h-4 mr-2', isRefreshing ? 'animate-spin' : '')}
                                                />
                                                {isRefreshing ? 'Refreshing...' : 'Refresh Devices'}
                                            </Button>
                                        </div>
                                    )}
                                </div>
                            </CardContent>
                        </Card>
                    </div>
                );
            })}
        </div>
    );
};

export default Settings;

import { invoke } from '@tauri-apps/api/core';
import { Setting, categories } from '@/lib/types';
import { SettingsItem } from '@/components/settings-item';

const settings: Setting[] = [
    {
        name: 'Clip name formatting',
        description:
            'The format of the clip name. You can use the following variables: %Y, %y, %m, %d, %H, %I, %M, %S, %f, %p',
        tooltip:
            '%Y: Full year (2025), %y: Short year (25), %m: Month (01-12), %d: Day of the month (01-31), %H: Hour (00-23), %I: Hour (01-12), %M: Minute (00-59), %S: Second (00-59), %f: Milliseconds (237), %p: AM/PM',
        type: 'string',
        defaultValue: '%Y%m%d_%H%M%S',
        storageKey: 'clip_name_formatting',
        category: categories.general,
    },
];

const handleSave = async (key: string, value: string | number | boolean) => {
    console.log(1, key, value);
    await invoke('update_setting', { key, value });
};

const pullSettings = async () => {
    const res = await invoke('get_settings');
    console.log(2, res);
};

const Settings = () => {
    return (
        <div className='w-full h-full flex flex-col px-8 py-4'>
            <span className='text-2xl text-zinc-200 mt-4 mb-8 font-semibold'>Settings</span>
            {Object.values(categories).map((category) => (
                <div>
                    <span className='text-sm text-zinc-400'>{category}</span>
                    {settings
                        .filter((setting) => setting.category === category)
                        .map((setting, index) => (
                            <SettingsItem key={index} {...setting} handleSave={handleSave} />
                        ))}
                </div>
            ))}
        </div>
    );
};

export default Settings;

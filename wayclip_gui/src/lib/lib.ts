import { readFile } from '@tauri-apps/plugin-fs';

export const convertLength = (seconds: number): string => {
    const mins = Math.floor(seconds / 60);
    const secs = Math.floor(seconds % 60);
    const paddedSecs = secs.toString().padStart(2, '0');
    return `${mins}:${paddedSecs}`;
};

export const convertTime = (ts: string): string => {
    const date = new Date(ts);

    const time = date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit', hour12: false });
    const month = date.toLocaleString('en-US', { month: 'long' });
    const day = date.getDate();
    const year = date.getFullYear();

    return `${time}, ${month} ${day}, ${year}`;
};

export const convertSize = (bytes: number, decimals = 2): string => {
    if (bytes === 0) return '0 Bytes';

    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];

    const i = Math.floor(Math.log(bytes) / Math.log(k));
    const value = bytes / Math.pow(k, i);

    return `${parseFloat(value.toFixed(decimals))} ${sizes[i]}`;
};

export const getPreview = async (path: string): Promise<string> => {
    const bytes = await readFile(path);
    const blob = new Blob([new Uint8Array(bytes)], { type: 'video/mp4' });
    return URL.createObjectURL(blob);
};

export const convertName = (input: string, mode: 'displayToStore' | 'storeToDisplay', defaultExt = '.mp4'): string => {
    if (mode === 'displayToStore') {
        const base = input.replace(/\s+/g, '_').replace(/[^a-zA-Z0-9_\-]/g, '');
        return base.endsWith(defaultExt) ? base : base + defaultExt;
    } else {
        const nameWithoutExt = input.replace(/\.[^/.]+$/, '');
        return nameWithoutExt.replace(/_/g, ' ');
    }
};

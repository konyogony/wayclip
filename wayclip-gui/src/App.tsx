// import { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './globals.css';

function App() {
    async function test_unix_socket() {
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        await invoke('test_unix_socket');
    }

    return (
        <main>
            <div className='flex flex-col items-center justify-center min-h-screen'>
                test
                <button
                    className='bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded'
                    onClick={() => test_unix_socket}
                >
                    Test Unix Socket
                </button>
            </div>
        </main>
    );
}

export default App;

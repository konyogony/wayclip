import React from 'react';
import ReactDOM from 'react-dom/client';
import Layout from '@/layouts/main-layout';
import Settings from '@/pages/settings';
import Home from '@/pages/home';
import { BrowserRouter, Routes, Route } from 'react-router';
import '@/styles/globals.css';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <BrowserRouter>
            <Routes>
                <Route element={<Layout />}>
                    <Route index path='/' element={<Home />} />
                    <Route path='/settings' element={<Settings />} />
                </Route>
            </Routes>
        </BrowserRouter>
    </React.StrictMode>,
);

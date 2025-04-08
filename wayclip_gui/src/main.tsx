import React from 'react';
import ReactDOM from 'react-dom/client';
import Layout from '@/layouts/main-layout';
import { BrowserRouter, Routes, Route } from 'react-router';
import '@/styles/globals.css';
import { routes } from '@/lib/routes';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <BrowserRouter>
            <Routes>
                <Route element={<Layout />}>
                    {routes.map((v, i) => (
                        <Route key={i} path={v.path} index={v.path === '/'} element={v.element} />
                    ))}
                </Route>
            </Routes>
        </BrowserRouter>
    </React.StrictMode>,
);

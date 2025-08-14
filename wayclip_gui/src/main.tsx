import React from 'react';
import ReactDOM from 'react-dom/client';
import Layout from '@/layouts/main-layout';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Routes, Route } from 'react-router';
import { routes } from '@/lib/routes';
import '@/styles/globals.css';

const queryClient = new QueryClient();

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
    <React.StrictMode>
        <QueryClientProvider client={queryClient}>
            <BrowserRouter>
                <Routes>
                    <Route element={<Layout />}>
                        {routes.map((v, i) => (
                            <Route key={i} path={v.path} index={v.path === '/'} element={v.element} />
                        ))}
                    </Route>
                </Routes>
            </BrowserRouter>
        </QueryClientProvider>
    </React.StrictMode>,
);

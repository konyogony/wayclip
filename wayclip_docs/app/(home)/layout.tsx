import { HomeLayout } from 'fumadocs-ui/layouts/home';
import { baseOptions } from '@/lib/layout.shared';
import type { Metadata } from 'next';

export const metadata: Metadata = {
    title: 'Wayclip - Instant Replay for Wayland',
    description:
        'Professional instant replay tool for Wayland/Linux. Capture, review, and share your screen recordings with ease.',
};

export default function Layout({ children }: LayoutProps<'/'>) {
    return <HomeLayout {...baseOptions()}>{children}</HomeLayout>;
}

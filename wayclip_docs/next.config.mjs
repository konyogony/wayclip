import { createMDX } from 'fumadocs-mdx/next';

const withMDX = createMDX();

/** @type {import('next').NextConfig} */
const config = {
    reactStrictMode: true,
    output: 'standalone',
    async rewrites() {
        return [{ source: '/api/search', destination: '/fd/search' }];
    },
};

export default withMDX(config);

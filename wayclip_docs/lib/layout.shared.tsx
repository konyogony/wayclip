import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export const baseOptions = (): BaseLayoutProps => {
    return {
        nav: {
            title: <>Wayclip</>,
        },
        githubUrl: 'https://github.com/konyogony/wayclip',
        themeSwitch: { enabled: false },
        links: [
            {
                text: 'Docs',
                url: '/docs',
                active: 'nested-url',
            },
        ],
    };
};

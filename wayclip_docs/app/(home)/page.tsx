import Link from 'next/link';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import {
    CheckCircle,
    Terminal,
    Monitor,
    Github,
    Download,
    Clock,
    FileVideo,
    Share2,
    Cloud,
    ArrowRight,
    Star,
    Users,
    Code,
    Package,
} from 'lucide-react';

export default function HomePage() {
    return (
        <div className='min-h-screen bg-background'>
            <section className='relative py-24 px-4'>
                <div className='container mx-auto text-center max-w-4xl'>
                    <div className='flex items-center justify-center gap-3 mb-8'>
                        <Badge variant='secondary'>
                            <Package className='w-4 h-4 mr-2' />
                            Beta Version
                        </Badge>
                    </div>

                    <h1 className='text-5xl md:text-7xl font-bold mb-6 text-balance tracking-tight'>
                        Instant Replay for <span className='text-primary'>Wayland</span>
                    </h1>

                    <p className='text-xl text-muted-foreground mb-12 text-pretty max-w-2xl mx-auto leading-relaxed'>
                        Capture and replay your screen instantly on Linux. Built for the modern desktop with Wayland and
                        PipeWire.
                    </p>

                    <div className='flex flex-col sm:flex-row gap-4 justify-center mb-16'>
                        <Button className='px-8 py-3' variant={'shadcn'}>
                            <Download className='w-5 h-5 mr-2' />
                            Download App
                            <ArrowRight className='w-4 h-4 ml-2' />
                        </Button>
                        <Button variant='outline' className='px-8 py-3 bg-transparent'>
                            <Terminal className='w-5 h-5 mr-2' />
                            Install CLI
                        </Button>
                    </div>

                    <div className='flex items-center justify-center gap-6 text-sm text-muted-foreground'>
                        <div className='flex items-center gap-2'>
                            <Star className='w-4 h-4 text-primary' />
                            <span>Soon available on AUR and Nix</span>
                        </div>
                    </div>
                </div>
            </section>

            <section className='py-20 px-4 bg-muted/30'>
                <div className='container mx-auto max-w-6xl'>
                    <div className='text-center mb-16'>
                        <h2 className='text-4xl font-bold mb-4 text-balance'>Core Features</h2>
                        <p className='text-lg text-muted-foreground max-w-2xl mx-auto'>
                            Everything you need for instant screen recording and replay on Linux.
                        </p>
                    </div>

                    <div className='grid md:grid-cols-2 lg:grid-cols-3 gap-8'>
                        {[
                            {
                                icon: Clock,
                                title: 'Instant Replay',
                                description:
                                    'Buffer your screen and save the last few moments with a quick hotkey press.',
                            },
                            {
                                icon: Monitor,
                                title: 'Wayland Native',
                                description:
                                    'Designed specifically for modern Linux desktops using Wayland and PipeWire.',
                            },
                            {
                                icon: FileVideo,
                                title: 'Built-in Editor',
                                description:
                                    'Trim and edit your clips with a simple, integrated editor before sharing.',
                            },
                            {
                                icon: Cloud,
                                title: 'Cloud Storage',
                                description:
                                    'Optional cloud storage for easy sharing. Keep recordings local or upload to share.',
                            },
                            {
                                icon: Share2,
                                title: 'Easy Sharing',
                                description:
                                    'Generate shareable links instantly. Perfect for bug reports and collaboration.',
                            },
                            {
                                icon: Code,
                                title: 'Selfhostable',
                                description:
                                    'Our API is fully selfhostable, check out the documentation for more information.',
                            },
                        ].map((feature) => (
                            <Card key={feature.title} className='border-0 shadow-sm hover:shadow-md transition-shadow'>
                                <CardHeader className='pb-4'>
                                    <feature.icon className='w-10 h-10 text-primary mb-4' />
                                    <CardTitle className='text-xl'>{feature.title}</CardTitle>
                                    <CardDescription className='text-base leading-relaxed'>
                                        {feature.description}
                                    </CardDescription>
                                </CardHeader>
                            </Card>
                        ))}
                    </div>
                </div>
            </section>

            <section className='py-20 px-4'>
                <div className='container mx-auto max-w-6xl'>
                    <div className='text-center mb-16'>
                        <h2 className='text-4xl font-bold mb-4 text-balance'>Simple Storage Pricing</h2>
                        <p className='text-lg text-muted-foreground max-w-2xl mx-auto'>
                            Wayclip is completely free. Only pay for cloud storage if you want more storage for sharing
                            your clips.
                        </p>
                    </div>

                    <div className='grid md:grid-cols-2 lg:grid-cols-4 gap-6'>
                        {[
                            {
                                name: 'Free',
                                storage: '2GB',
                                price: '$0',
                                description: 'Perfect for trying out cloud sharing',
                                features: ['2GB cloud storage', 'Unlimited local recordings', 'Basic sharing links'],
                                popular: false,
                            },
                            {
                                name: 'Basic',
                                storage: '50GB',
                                price: '$3.99',
                                description: 'Great for regular content creators',
                                features: ['50GB cloud storage'],
                                popular: true,
                            },
                            {
                                name: 'Plus',
                                storage: '200GB',
                                price: '$6.99',
                                description: 'For power users and teams',
                                features: ['200GB cloud storage'],
                                popular: false,
                            },
                            {
                                name: 'Pro',
                                storage: '1TB',
                                price: '$14.99',
                                description: 'For professional workflows',
                                features: ['1TB cloud storage'],
                                popular: false,
                            },
                        ].map((plan) => (
                            <Card
                                key={plan.name}
                                className={`relative ${plan.popular ? 'border-primary shadow-lg scale-105' : 'border-border'}`}
                            >
                                {plan.popular && (
                                    <div className='absolute -top-3 left-1/2 transform -translate-x-1/2'>
                                        <Badge className='bg-primary text-primary-foreground px-4 py-1'>
                                            Most Popular
                                        </Badge>
                                    </div>
                                )}
                                <CardHeader className='text-center pb-6'>
                                    <CardTitle className='text-2xl mb-2'>{plan.storage}</CardTitle>
                                    <div className='text-4xl font-bold mb-2'>{plan.price}</div>
                                    <CardDescription>{plan.description}</CardDescription>
                                </CardHeader>
                                <CardContent className='h-full flex flex-col gap-4'>
                                    <ul className='space-y-3'>
                                        {plan.features.map((feature) => (
                                            <li key={feature} className='flex items-center gap-3'>
                                                <CheckCircle className='w-4 h-4 text-primary flex-shrink-0' />
                                                <span className='text-sm'>{feature}</span>
                                            </li>
                                        ))}
                                    </ul>
                                    <Button asChild className='px-6 py-3 mt-auto' variant={'outline'} size={'sm'}>
                                        <a href='dash.wayclip.com'>
                                            Purchase <ArrowRight />
                                        </a>
                                    </Button>
                                </CardContent>
                            </Card>
                        ))}
                    </div>
                </div>
            </section>

            <section className='py-20 px-4'>
                <div className='container mx-auto max-w-4xl text-center'>
                    <div className='bg-muted/50 rounded-2xl p-12'>
                        <Github className='w-16 h-16 text-primary mx-auto mb-6' />
                        <h2 className='text-4xl font-bold mb-4 text-balance'>
                            Open Source
                            <br /> & Community Driven
                        </h2>
                        <p className='text-lg text-muted-foreground mb-8 max-w-2xl mx-auto'>
                            Wayclip is built right in the open. Check out the code, report issues, or contribute to make
                            it better.
                        </p>
                        <div className='flex flex-col sm:flex-row gap-4 justify-center mb-12'>
                            <Button variant='outline' className='px-6 py-3 bg-transparent' asChild>
                                <a
                                    href='https://github.com/konyogony/wayclip'
                                    target='_blank'
                                    rel='noopener norefferer'
                                >
                                    <Github className='w-5 h-5 mr-2' />
                                    View on GitHub
                                </a>
                            </Button>
                            <Button variant='outline' className='px-6 py-3 bg-transparent' asChild>
                                <a href='https://discord.gg/BrXAHknFE6' target='_blank' rel='noopener norefferer'>
                                    <Users className='w-5 h-5 mr-2' />
                                    Join Community
                                </a>
                            </Button>
                        </div>

                        <div className='grid grid-cols-1 md:grid-cols-3 gap-8'>
                            <div className='text-center'>
                                <div className='text-3xl font-bold text-primary mb-2'>1</div>
                                <div className='text-sm text-muted-foreground'>Contributors</div>
                            </div>
                            <div className='text-center'>
                                <div className='text-3xl font-bold text-primary mb-2'>MIT</div>
                                <div className='text-sm text-muted-foreground'>Open Source License</div>
                            </div>
                            <div className='text-center'>
                                <div className='text-3xl font-bold text-primary mb-2'>∞</div>
                                <div className='text-sm text-muted-foreground'>Downloads</div>
                            </div>
                        </div>
                    </div>
                </div>
            </section>

            <footer className='border-t py-12 px-4'>
                <div className='container mx-auto max-w-6xl'>
                    <div className='grid grid-cols-1 md:grid-cols-4 gap-8 mb-8'>
                        <div className='md:col-span-2'>
                            <div className='flex items-center space-x-2 mb-4'>
                                <span className='font-bold text-xl'>Wayclip</span>
                            </div>
                            <p className='text-muted-foreground mb-4 max-w-md'>
                                An open-source instant replay tool for modern Linux desktops. Built with ❤️ for the
                                Linux community.
                            </p>
                        </div>
                        <div>
                            <h4 className='font-semibold mb-4'>Product</h4>
                            <ul className='space-y-2 text-sm text-muted-foreground'>
                                <li>
                                    <a href='#Features' className='hover:text-foreground transition-colors'>
                                        Features
                                    </a>
                                </li>
                                <li>
                                    <a href='#Pricing' className='hover:text-foreground transition-colors'>
                                        Pricing
                                    </a>
                                </li>
                            </ul>
                        </div>
                        <div>
                            <h4 className='font-semibold mb-4'>Community</h4>
                            <ul className='space-y-2 text-sm text-muted-foreground'>
                                <li>
                                    <a
                                        href='https://github.com/konyogony/wayclip'
                                        className='hover:text-foreground transition-colors'
                                    >
                                        GitHub
                                    </a>
                                </li>
                                <li>
                                    <Link
                                        href={'/docs/contributing'}
                                        className='hover:text-foreground transition-colors'
                                    >
                                        Contributing
                                    </Link>
                                </li>
                                <li>
                                    <a
                                        href='https://discord.gg/BrXAHknFE6'
                                        className='hover:text-foreground transition-colors'
                                    >
                                        Discord
                                    </a>
                                </li>
                                <li>
                                    <Link href={'/docs'} className='hover:text-foreground transition-colors'>
                                        Documentation
                                    </Link>
                                </li>
                            </ul>
                        </div>
                    </div>
                    <div className='border-t pt-8 text-center'>
                        <p className='text-sm text-muted-foreground'>
                            &copy; {new Date().getFullYear()} Wayclip. Open source software licensed under MIT.
                        </p>
                    </div>
                </div>
            </footer>
        </div>
    );
}

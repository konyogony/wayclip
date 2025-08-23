import { useState, useEffect, useRef, RefObject } from 'react';

export const useLazyLoad = <T extends HTMLElement>(): [RefObject<T | null>, boolean] => {
    const [inView, setInView] = useState(false);

    const ref = useRef<T | null>(null);

    useEffect(() => {
        const element = ref.current;

        const observer = new IntersectionObserver(
            ([entry]) => {
                if (entry.isIntersecting) {
                    setInView(true);
                    if (element) {
                        observer.unobserve(element);
                    }
                }
            },
            {
                rootMargin: '100px',
            },
        );

        if (element) {
            observer.observe(element);
        }

        return () => {
            if (element) {
                observer.unobserve(element);
            }
        };
    }, []);

    return [ref, inView];
};

'use client';

import { useState, useCallback } from 'react';

const DEFAULT_PLACEHOLDER = '/images/logo.png';

export function useImageWithFallback(initialSrc, placeholder = DEFAULT_PLACEHOLDER) {
    const [imageError, setImageError] = useState(false);
    const [imageLoaded, setImageLoaded] = useState(false);

    const handleError = useCallback(() => {
        if (!imageError) {
            setImageError(true);
        }
    }, [imageError]);

    const handleLoad = useCallback(() => {
        if (!imageLoaded) {
            setImageLoaded(true);
        }
    }, [imageLoaded]);

    const currentSrc = imageError || !initialSrc ? placeholder : initialSrc;
    const isPlaceholder = currentSrc === placeholder;

    return {
        src: currentSrc,
        isPlaceholder,
        imageLoaded,
        handleError,
        handleLoad
    };
}

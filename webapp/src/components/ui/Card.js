import React from 'react';

export function Card({ children, className = '' }) {
    return (
        <div className={`bg-dark-900/80 border border-dark-800/50 rounded-lg overflow-hidden ${className}`}>
            {children}
        </div>
    );
}

export function CardHeader({ children, className = '' }) {
    return (
        <div className={`px-6 py-4 border-b border-dark-800/50 ${className}`}>
            {children}
        </div>
    );
}

export function CardBody({ children, className = '' }) {
    return (
        <div className={`px-6 py-4 ${className}`}>
            {children}
        </div>
    );
}

export function CardFooter({ children, className = '' }) {
    return (
        <div className={`px-6 py-3 border-t border-dark-800/50 ${className}`}>
            {children}
        </div>
    );
}

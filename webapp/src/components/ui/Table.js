import React from 'react';

export function Table({ children, className = '' }) {
    return (
        <table className={`min-w-full divide-y divide-dark-700 ${className}`}>
            {children}
        </table>
    );
}

export function TableHeader({ children, className = '' }) {
    return (
        <thead className={`bg-dark-800/50 ${className}`}>
            {children}
        </thead>
    );
}

export function TableBody({ children, className = '' }) {
    return (
        <tbody className={`bg-dark-900/30 divide-y divide-dark-800 ${className}`}>
            {children}
        </tbody>
    );
}

export function TableRow({ children, className = '' }) {
    return (
        <tr className={`hover:bg-dark-800/50 transition-colors ${className}`}>
            {children}
        </tr>
    );
}

export function TableCell({ children, className = '', colSpan }) {
    return (
        <td className={`px-6 py-4 text-sm ${className}`} colSpan={colSpan}>
            {children}
        </td>
    );
}

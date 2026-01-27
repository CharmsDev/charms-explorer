'use client';

import { Button } from './ui/Button';

export default function Pagination({
    currentPage,
    totalPages,
    totalItems,
    itemsPerPage,
    onPageChange,
    maxVisiblePages = 7
}) {
    const calculatedTotalPages = Math.max(Math.ceil(totalItems / itemsPerPage), 1);
    const effectiveTotalPages = totalPages || calculatedTotalPages;

    const renderPageNumbers = () => {
        const pageNumbers = [];

        let startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
        let endPage = Math.min(effectiveTotalPages, startPage + maxVisiblePages - 1);

        if (endPage - startPage + 1 < maxVisiblePages) {
            startPage = Math.max(1, endPage - maxVisiblePages + 1);
        }

        if (startPage > 1) {
            pageNumbers.push(
                <Button
                    key={1}
                    onClick={() => onPageChange(1)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === 1 ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    1
                </Button>
            );

            if (startPage > 2) {
                pageNumbers.push(
                    <span key="ellipsis1" className="px-1">...</span>
                );
            }
        }

        for (let i = startPage; i <= endPage; i++) {
            pageNumbers.push(
                <Button
                    key={i}
                    onClick={() => onPageChange(i)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === i ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    {i}
                </Button>
            );
        }

        if (endPage < effectiveTotalPages) {
            if (endPage < effectiveTotalPages - 1) {
                pageNumbers.push(
                    <span key="ellipsis2" className="px-1">...</span>
                );
            }

            pageNumbers.push(
                <Button
                    key={effectiveTotalPages}
                    onClick={() => onPageChange(effectiveTotalPages)}
                    className={`w-8 h-8 p-0 text-sm font-bold ${currentPage === effectiveTotalPages ? 'bg-primary-700 text-white' : 'bg-dark-700 text-dark-200'}`}
                >
                    {effectiveTotalPages}
                </Button>
            );
        }

        return pageNumbers;
    };

    return (
        <div className="container mx-auto px-4 py-6">
            <div className="flex flex-col items-center">
                <div className="text-sm text-dark-400 mb-2">
                    Page {currentPage} of {effectiveTotalPages}
                </div>

                <div className="flex items-center space-x-2 flex-wrap">
                    <Button
                        onClick={() => onPageChange(1)}
                        disabled={currentPage === 1}
                        className="px-3 py-1"
                    >
                        First
                    </Button>
                    <Button
                        onClick={() => onPageChange(currentPage - 1)}
                        disabled={currentPage === 1}
                        className="px-3 py-1"
                    >
                        Previous
                    </Button>

                    <div className="flex items-center space-x-1 mx-2 bg-dark-800/50 px-2 py-1 rounded-lg">
                        {renderPageNumbers()}
                    </div>

                    <Button
                        onClick={() => onPageChange(currentPage + 1)}
                        disabled={currentPage >= effectiveTotalPages}
                        className="px-3 py-1"
                    >
                        Next
                    </Button>
                    <Button
                        onClick={() => onPageChange(effectiveTotalPages)}
                        disabled={currentPage >= effectiveTotalPages}
                        className="px-3 py-1"
                    >
                        Last
                    </Button>
                </div>
            </div>
        </div>
    );
}

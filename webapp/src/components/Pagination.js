'use client';

export default function Pagination({
    currentPage,
    totalPages,
    totalItems,
    itemsPerPage,
    onPageChange,
    maxVisiblePages = 7
}) {
    const effectiveTotalPages = totalPages || Math.max(Math.ceil((totalItems || 0) / itemsPerPage), 1);

    if (effectiveTotalPages <= 1) return null;

    const handlePageChange = (page) => {
        if (page < 1 || page > effectiveTotalPages || page === currentPage) return;
        onPageChange(page);
        window.scrollTo({ top: 0, behavior: 'smooth' });
    };

    // Calculate visible page range
    let startPage = Math.max(1, currentPage - Math.floor(maxVisiblePages / 2));
    let endPage = Math.min(effectiveTotalPages, startPage + maxVisiblePages - 1);
    if (endPage - startPage + 1 < maxVisiblePages) {
        startPage = Math.max(1, endPage - maxVisiblePages + 1);
    }

    const pages = [];
    // First page + ellipsis
    if (startPage > 1) {
        pages.push(1);
        if (startPage > 2) pages.push('...');
    }
    // Visible range
    for (let i = startPage; i <= endPage; i++) pages.push(i);
    // Ellipsis + last page
    if (endPage < effectiveTotalPages) {
        if (endPage < effectiveTotalPages - 1) pages.push('...');
        pages.push(effectiveTotalPages);
    }

    const btnBase = "px-3 py-1.5 text-sm font-medium rounded-md transition-colors disabled:opacity-40 disabled:cursor-not-allowed";
    const btnNav = `${btnBase} bg-dark-800 text-dark-200 hover:bg-dark-700 hover:text-white`;
    const btnPage = (active) => `${btnBase} min-w-[2rem] ${active ? 'bg-primary-600 text-white' : 'bg-dark-800 text-dark-300 hover:bg-dark-700 hover:text-white'}`;

    return (
        <div className="flex flex-col items-center gap-2 py-6">
            <span className="text-xs text-dark-500">Page {currentPage} of {effectiveTotalPages}</span>
            <div className="flex items-center gap-1">
                <button className={btnNav} disabled={currentPage === 1} onClick={() => handlePageChange(1)}>First</button>
                <button className={btnNav} disabled={currentPage === 1} onClick={() => handlePageChange(currentPage - 1)}>Prev</button>
                <div className="flex items-center gap-1 mx-1">
                    {pages.map((p, i) =>
                        p === '...' ? (
                            <span key={`e${i}`} className="px-1 text-dark-500">...</span>
                        ) : (
                            <button key={p} className={btnPage(p === currentPage)} onClick={() => handlePageChange(p)}>{p}</button>
                        )
                    )}
                </div>
                <button className={btnNav} disabled={currentPage >= effectiveTotalPages} onClick={() => handlePageChange(currentPage + 1)}>Next</button>
                <button className={btnNav} disabled={currentPage >= effectiveTotalPages} onClick={() => handlePageChange(effectiveTotalPages)}>Last</button>
            </div>
        </div>
    );
}

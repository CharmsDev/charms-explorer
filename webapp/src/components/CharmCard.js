'use client';

import { useState } from 'react';
import Link from 'next/link';
import { likeCharm, unlikeCharm } from '../services/apiServices';

export default function CharmCard({ charm }) {
    const [imageError, setImageError] = useState(false);
    const [isHovered, setIsHovered] = useState(false);
    const [likesCount, setLikesCount] = useState(charm.likes);
    const [isLiked, setIsLiked] = useState(charm.userLiked);
    const [isLikeLoading, setIsLikeLoading] = useState(false);
    const placeholderImage = "/images/logo.png";

    // Determine if the charm is an NFT
    const isNftCharm = charm.type === 'nft';

    // Format date
    const formattedDate = new Date(charm.createdAt).toLocaleDateString('en-US', {
        year: 'numeric',
        month: 'short',
        day: 'numeric'
    });

    // Get type-specific icon (but keep consistent color scheme)
    const getTypeDetails = () => {
        switch (charm.type) {
            case 'nft':
                return {
                    color: 'from-purple-500 to-indigo-700',
                    bgColor: 'bg-purple-600/10',
                    icon: (
                        <svg className="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
                        </svg>
                    )
                };
            case 'token':
                return {
                    color: 'from-bitcoin-500 to-bitcoin-700',
                    bgColor: 'bg-bitcoin-600/10',
                    icon: (
                        <svg className="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M12 8c-1.657 0-3 .895-3 2s1.343 2 3 2 3 .895 3 2-1.343 2-3 2m0-8c1.11 0 2.08.402 2.599 1M12 8V7m0 1v8m0 0v1m0-1c-1.11 0-2.08-.402-2.599-1M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                        </svg>
                    )
                };
            case 'dapp':
                return {
                    color: 'from-emerald-500 to-teal-700',
                    bgColor: 'bg-emerald-600/10',
                    icon: (
                        <svg className="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
                        </svg>
                    )
                };
            case 'spell':
                return {
                    color: 'from-rose-500 to-pink-700',
                    bgColor: 'bg-rose-600/10',
                    icon: (
                        <svg className="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z"></path>
                        </svg>
                    )
                };
            default:
                return {
                    color: 'from-primary-500 to-primary-700',
                    bgColor: 'bg-primary-600/10',
                    icon: (
                        <svg className="w-3 h-3 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M7 7h.01M7 3h5c.512 0 1.024.195 1.414.586l7 7a2 2 0 010 2.828l-7 7a2 2 0 01-2.828 0l-7-7A1.994 1.994 0 013 12V7a4 4 0 014-4z"></path>
                        </svg>
                    )
                };
        }
    };

    const typeDetails = getTypeDetails();

    const isPlaceholder = !imageError && charm.image === placeholderImage;

    return (
        <div
            className="card card-hover flex flex-col h-full transform transition-all duration-300 bg-gradient-to-br from-dark-800 to-dark-900"
            onMouseEnter={() => setIsHovered(true)}
            onMouseLeave={() => setIsHovered(false)}
        >
            {/* Image section with gradient overlay */}
            <Link href={`/asset/${charm.id}`} className="relative block w-full h-48 bg-dark-900 overflow-hidden group">
                <img
                    src={!imageError ? charm.image : placeholderImage}
                    alt={charm.name}
                    className={`w-full h-full object-cover transition-all duration-500 ${!isPlaceholder ? (isHovered ? 'scale-110' : 'scale-100') : 'opacity-60 group-hover:opacity-100'}`}
                    onError={() => setImageError(true)}
                />
                <div className={`absolute inset-0 bg-gradient-to-t from-dark-900 to-transparent opacity-60`}></div>

                {/* Type badge */}
                <div className={`absolute top-3 right-3 ${typeDetails.bgColor} px-2 py-1 rounded-md text-xs font-medium flex items-center`}>
                    {typeDetails.icon}
                    <span className={`bg-gradient-to-r ${typeDetails.color} bg-clip-text text-transparent`}>
                        {isNftCharm ? 'NFT' :
                            charm.type === 'token' ? 'Token' :
                                charm.type === 'dapp' ? 'dApp' :
                                    charm.type === 'spell' ? 'Spell' :
                                        charm.type}
                    </span>
                </div>
            </Link>

            <div className="p-5 flex-grow flex flex-col">
                <div className="flex justify-between items-start">
                    <div>
                        <Link href={`/asset/${charm.id}`} className="group">
                            <h3 className="font-bold text-lg text-white group-hover:text-primary-400 transition-colors">
                                {charm.name}
                            </h3>
                            <div className="h-0.5 w-0 bg-gradient-to-r from-primary-400 to-primary-600 group-hover:w-full transition-all duration-300"></div>
                        </Link>
                    </div>
                    {charm.type === 'token' && (
                        <div className="text-right">
                            <span className="text-lg font-semibold text-primary-400">{charm.remaining}</span>
                            <p className="text-xs text-dark-400">{charm.ticker}</p>
                        </div>
                    )}
                </div>

                {/* Description section */}
                {charm.description && (
                    <div className="mt-3">
                        <p className="text-sm text-dark-300 line-clamp-2">{charm.description}</p>
                    </div>
                )}

                {/* URL section for dApps */}
                {charm.url && (
                    <div className="mt-3">
                        <a
                            href={charm.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="inline-flex items-center text-sm text-primary-400 hover:text-primary-300 transition-colors"
                        >
                            <svg className="w-4 h-4 mr-1" fill="none" stroke="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth="2" d="M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"></path>
                            </svg>
                            Visit website
                        </a>
                    </div>
                )}

                {/* Stats section */}
                <div className="mt-4 pt-3 border-t border-dark-700 flex items-center justify-between">
                    <div className="flex items-center space-x-4">
                        <button
                            className={`flex items-center group ${isLikeLoading ? 'opacity-50 cursor-wait' : 'cursor-pointer'}`}
                            onClick={async (e) => {
                                e.preventDefault();
                                if (isLikeLoading) return;

                                setIsLikeLoading(true);
                                try {
                                    if (isLiked) {
                                        const response = await unlikeCharm(charm.id);
                                        setLikesCount(response.likes_count);
                                        setIsLiked(false);
                                    } else {
                                        const response = await likeCharm(charm.id);
                                        setLikesCount(response.likes_count);
                                        setIsLiked(true);
                                    }
                                } catch (error) {
                                    console.error('Error toggling like:', error);
                                } finally {
                                    setIsLikeLoading(false);
                                }
                            }}
                            disabled={isLikeLoading}
                        >
                            <svg
                                xmlns="http://www.w3.org/2000/svg"
                                className={`h-4 w-4 transition-colors ${isLiked ? 'text-primary-400 fill-primary-400' : 'text-dark-500 group-hover:text-primary-400'}`}
                                viewBox="0 0 24 24"
                                stroke="currentColor"
                            >
                                <path
                                    strokeLinecap="round"
                                    strokeLinejoin="round"
                                    strokeWidth={isLiked ? 1 : 2}
                                    d="M4.318 6.318a4.5 4.5 0 000 6.364L12 20.364l7.682-7.682a4.5 4.5 0 00-6.364-6.364L12 7.636l-1.318-1.318a4.5 4.5 0 00-6.364 0z"
                                />
                            </svg>
                            <span className={`ml-1 text-sm transition-colors ${isLiked ? 'text-primary-400' : 'text-dark-400 group-hover:text-primary-400'}`}>
                                {likesCount}
                            </span>
                        </button>
                        <div className="flex items-center group cursor-not-allowed opacity-50" title="Comments not yet implemented">
                            <svg xmlns="http://www.w3.org/2000/svg" className="h-4 w-4 text-dark-700" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M8 12h.01M12 12h.01M16 12h.01M21 12c0 4.418-4.03 8-9 8a9.863 9.863 0 01-4.255-.949L3 20l1.395-3.72C3.512 15.042 3 13.574 3 12c0-4.418 4.03-8 9-8s9 3.582 9 8z" />
                            </svg>
                            <span className="ml-1 text-sm text-dark-700">{charm.comments}</span>
                        </div>
                    </div>
                    <div className="text-xs text-dark-400">{formattedDate}</div>
                </div>

                {/* UTXO ID (shortened) */}
                {charm.txid && (
                    <div className="mt-2 text-xs text-dark-500 font-mono truncate hover:text-primary-400 transition-colors cursor-pointer" title={`${charm.txid}:${charm.outputIndex || 0}`}>
                        {charm.txid.substring(0, 8)}...:{charm.outputIndex || 0}
                    </div>
                )}

                {/* Version tag if available */}
                {charm.version && (
                    <div className="mt-1 text-xs text-dark-500">
                        <span className="bg-dark-800 px-1.5 py-0.5 rounded">v{charm.version}</span>
                    </div>
                )}
            </div>
        </div>
    );
}

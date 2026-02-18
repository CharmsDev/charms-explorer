"use client";

export const runtime = "edge";

import React, { useState, useEffect } from "react";
import Link from "next/link";
import { useParams } from "next/navigation";
import {
  getAssetById,
  fetchAssetByAppId,
  fetchAssetHolders,
} from "../../../services/api";
import { parseSpellMetadata } from "../../../services/spellParser";
import {
  fetchReferenceNftByHash,
  extractHashFromAppId,
} from "../../../services/api/referenceNft";
import HoldersTab from "../../../components/HoldersTab";
import {
  AssetHero,
  AssetTechnicalDetails,
  TokenReferenceNFT,
  RawMetadata,
} from "../../../components/asset";

const PLACEHOLDER_IMAGE = "/images/logo.png";

export default function AssetDetailPage() {
  const { id } = useParams();
  const decodedId = id ? decodeURIComponent(id) : null;

  const [asset, setAsset] = useState(null);
  const [assetData, setAssetData] = useState(null);
  const [holdersData, setHoldersData] = useState(null);
  const [nftMetadata, setNftMetadata] = useState(null);
  const [spellImage, setSpellImage] = useState(null);
  const [isLoading, setIsLoading] = useState(true);
  const [imageLoading, setImageLoading] = useState(true);
  const [imageError, setImageError] = useState(false);
  const [activeTab, setActiveTab] = useState("details");

  // Phase 1: Load basic asset data (fast, ~0.2s via /assets?app_id=) — show the page immediately
  useEffect(() => {
    if (!decodedId) return;

    const loadAsset = async () => {
      try {
        setIsLoading(true);

        // Use the fast /assets endpoint (~0.2s) instead of /charms/by-charmid/ (~2.4s)
        const [assetResponse, holders] = await Promise.all([
          fetchAssetByAppId(decodedId).catch(() => null),
          fetchAssetHolders(decodedId).catch(() => null),
        ]);

        if (assetResponse) {
          // Map /assets response fields to the format the page expects
          const mapped = {
            id: assetResponse.id,
            app_id: assetResponse.app_id,
            name: assetResponse.name || "Unnamed Asset",
            type: assetResponse.asset_type,
            ticker: assetResponse.symbol,
            description: assetResponse.description,
            image: assetResponse.image_url,
            image_url: assetResponse.image_url,
            network: assetResponse.network,
            block_height: assetResponse.block_height,
            createdAt: assetResponse.created_at,
            verified: false,
          };
          setAsset(mapped);
          setAssetData(assetResponse);
        } else {
          // Fallback to slower /charms/by-charmid/ if /assets has no data
          const data = await getAssetById(decodedId);
          setAsset(data);
        }

        if (holders) setHoldersData(holders);
      } catch (error) {
        // Error handled - UI shows empty state
      } finally {
        setIsLoading(false);
      }
    };

    loadAsset();
  }, [decodedId]);

  // Phase 2: Lazy-load reference NFT image (can be slow)
  useEffect(() => {
    if (!asset) return;

    const loadImage = async () => {
      try {
        setImageLoading(true);
        const appId = asset?.app_id || asset?.id || decodedId;

        const hash = extractHashFromAppId(appId);
        if (hash) {
          const refNft = await fetchReferenceNftByHash(hash).catch(() => null);
          if (refNft) {
            setNftMetadata(refNft);
            const hasOwnImage =
              asset?.image &&
              asset.image !== "/images/logo.png" &&
              asset.image !== PLACEHOLDER_IMAGE;
            const hasOwnImageUrl =
              asset?.image_url &&
              asset.image_url !== "/images/logo.png" &&
              asset.image_url !== PLACEHOLDER_IMAGE;
            if (!hasOwnImage && !hasOwnImageUrl && refNft.image_url) {
              setSpellImage(refNft.image_url);
            }
          }
        }
      } catch (error) {
        // Image load failed silently
      } finally {
        setImageLoading(false);
      }
    };

    loadImage();
  }, [asset, decodedId]);

  // Loading state
  if (isLoading) {
    return (
      <div className="container mx-auto px-4 py-12">
        <div className="max-w-4xl mx-auto animate-pulse">
          <div className="h-8 bg-gray-700 rounded w-1/3 mb-6" />
          <div className="h-96 bg-gray-700 rounded mb-6" />
          <div className="h-4 bg-gray-700 rounded w-full mb-2" />
          <div className="h-4 bg-gray-700 rounded w-3/4" />
        </div>
      </div>
    );
  }

  // Not found state
  if (!asset) {
    return (
      <div className="container mx-auto px-4 py-12 text-center">
        <h1 className="text-2xl font-bold mb-4">Asset Not Found</h1>
        <p className="mb-6">
          The asset you're looking for doesn't exist or has been removed.
        </p>
        <Link
          href="/"
          className="bg-indigo-600 text-white px-4 py-2 rounded-md hover:bg-indigo-700"
        >
          Return to Home
        </Link>
      </div>
    );
  }

  // Computed values
  const formattedDate = new Date(
    asset.createdAt || assetData?.created_at,
  ).toLocaleDateString("en-US", {
    year: "numeric",
    month: "long",
    day: "numeric",
  });

  const decimals = assetData?.decimals || 8;
  const assetSupply = assetData?.total_supply
    ? Number(assetData.total_supply)
    : 0;
  const holdersSupply = holdersData?.total_supply
    ? Number(holdersData.total_supply)
    : 0;
  const totalSupply =
    (assetSupply > 0 ? assetSupply : holdersSupply) / Math.pow(10, decimals);

  const spellMetadata = parseSpellMetadata(asset);

  const getDisplayImage = () => {
    if (imageError) return PLACEHOLDER_IMAGE;
    // Both tokens and NFTs can use spell image from reference NFT
    return spellImage || asset.image || asset.image_url || PLACEHOLDER_IMAGE;
  };

  const typeLabels = { nft: "NFTs", token: "Tokens", dapp: "dApps" };

  return (
    <div className="container mx-auto px-4 py-8">
      <div className="max-w-6xl mx-auto">
        {/* Breadcrumb */}
        <div className="flex items-center text-sm text-dark-400 mb-6">
          <Link href="/" className="hover:text-primary-400">
            Home
          </Link>
          <span className="mx-2">/</span>
          <Link
            href={`/?type=${asset.type}`}
            className="hover:text-primary-400"
          >
            {typeLabels[asset.type] || "Assets"}
          </Link>
          <span className="mx-2">/</span>
          <span className="font-medium text-white">{asset.name}</span>
        </div>

        {/* Hero Section */}
        <AssetHero
          asset={asset}
          displayImage={getDisplayImage()}
          totalSupply={totalSupply}
          decimals={decimals}
          formattedDate={formattedDate}
          onImageError={() => setImageError(true)}
          description={asset.description || nftMetadata?.description}
          imageLoading={imageLoading}
        />

        {/* Token: Reference NFT Link */}
        {asset.type === "token" && (
          <TokenReferenceNFT nftMetadata={nftMetadata} />
        )}

        {/* Tabs */}
        <div className="mb-8">
          <div className="border-b border-dark-700">
            <nav className="-mb-px flex space-x-8">
              <button
                onClick={() => setActiveTab("details")}
                className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                  activeTab === "details"
                    ? "border-primary-500 text-primary-400"
                    : "border-transparent text-dark-400 hover:text-white"
                }`}
              >
                Details
              </button>
              {asset.type !== "token" && (
                <button
                  onClick={() => setActiveTab("metadata")}
                  className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                    activeTab === "metadata"
                      ? "border-primary-500 text-primary-400"
                      : "border-transparent text-dark-400 hover:text-white"
                  }`}
                >
                  Metadata
                </button>
              )}
              {asset.type === "token" && (
                <button
                  onClick={() => setActiveTab("holders")}
                  className={`py-4 px-1 border-b-2 font-medium text-sm transition-colors ${
                    activeTab === "holders"
                      ? "border-primary-500 text-primary-400"
                      : "border-transparent text-dark-400 hover:text-white"
                  }`}
                >
                  Holders ({holdersData?.total_holders || 0})
                </button>
              )}
            </nav>
          </div>

          <div className="mt-6">
            {activeTab === "details" && (
              <AssetTechnicalDetails asset={asset} holdersData={holdersData} />
            )}
            {activeTab === "metadata" && asset.type !== "token" && (
              <RawMetadata
                asset={asset}
                nftMetadata={nftMetadata}
                spellMetadata={spellMetadata}
              />
            )}
            {activeTab === "holders" && asset.type === "token" && (
              <HoldersTab appId={asset.app_id || asset.id} decimals={assetData?.decimals ?? 8} />
            )}
          </div>
        </div>

        {/* Back link */}
        <div className="flex items-center justify-end border-t border-dark-700 pt-6">
          <Link
            href={`/?type=${asset.type}`}
            className="text-primary-400 hover:text-primary-300"
          >
            ← Back to {typeLabels[asset.type] || "Assets"}
          </Link>
        </div>
      </div>
    </div>
  );
}

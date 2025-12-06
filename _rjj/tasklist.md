# Tareas Pendientes Indexer

## Problemas Actuales

1. ~~**Supply incorrecto**~~: ✅ RESUELTO - Assets ahora guardan supply correcto
2. ~~**Bigint overflow**~~: ✅ RESUELTO - Capped a i64::MAX/2
3. ~~**Metadatos NULL en NFTs**~~: ✅ RESUELTO - NFTs extraen metadata correctamente
4. ~~**Campos vacíos en assets**~~: ✅ RESUELTO - block_height, txid, vout_index se guardan
5. **Metadatos en Tokens**: ⏳ PENDIENTE - Tokens deben heredar metadata del NFT
6. **Stats holders negativos**: ⏳ PENDIENTE - charm_count negativo por lógica errónea

## Tareas Críticas

### 1. Detectar MINT vs TRANSFER correctamente

**Estado**: ⚠️ LÓGICA INCORRECTA - REQUIERE REIMPLEMENTACIÓN

**Problema actual:**

- Actualmente incrementamos supply por cada charm detectado (outputs)
- NO verificamos si es MINT (nuevo supply) o TRANSFER (supply sin cambios)
- Esto causa **doble conteo** del supply en transferencias

**Lógica correcta:**

1. **MINT**: `sum(inputs) < sum(outputs)` → Incrementar supply por la diferencia
2. **TRANSFER**: `sum(inputs) == sum(outputs)` → Supply sin cambios (NO incrementar)
3. **BURN**: `sum(inputs) > sum(outputs)` → Decrementar supply (no implementado aún)

**Implementación requerida:**

- Al procesar una transacción, sumar amounts de inputs (charms gastados)
- Sumar amounts de outputs (charms creados)
- Solo incrementar supply si `outputs > inputs` (mint neto)
- Archivos a modificar: `detection.rs` o `block_processor.rs`

**Prioridad**: 🔴 CRÍTICA - Sin esto el supply es incorrecto

### 2. Consolidar NFT + Token

**Estado**: ✅ IMPLEMENTADA | ✅ VERIFICADA (2025-12-07)

- Archivo: `detection.rs` líneas 212-276
- Archivo: `asset_repository.rs` líneas 187-228
- Tokens convierten `t/HASH` a `n/HASH` para referenciar el NFT
- Upsert incrementa `total_supply` del NFT cuando llega un token
- Verificado: NFT BRO con supply de 82 tokens acumulados
- Ver: Sesión 2025-12-07 checkpoint 33

### 3. Fix bigint overflow en stats_holders

**Estado**: ✅ IMPLEMENTADA | ✅ VERIFICADA

- Archivo: `stats_holders_repository.rs` líneas 28-33
- Cap `amount_delta` a `i64::MAX / 2` antes de insertar
- Ver: `_rjj/verificacion_3_bigint_overflow.md`
- Verificación: Indexer corre sin error `bigint out of range`

### 4. Extraer metadatos de NFT

**Estado**: ✅ IMPLEMENTADA | ✅ VERIFICADA

- Archivo: `detection.rs` líneas 146-193 (extracción metadata)
- Archivo: `detection.rs` líneas 195-231 (AssetSaveRequest)
- NFTs extraen metadata de `charm_json.native_data.tx.outs[0]["0"]`
- Campos extraídos: name, ticker/symbol, description, url/image_url, decimals
- [RJJ-TODO] supply_limit ignorado por ahora (documentado líneas 161-165)
- NFTs inician con total_supply = 0
- Ver: `_rjj/verificacion_4_nft_metadata.md`

**Verificación**:

```sql
-- NFT BRO correctamente guardado
SELECT * FROM assets WHERE app_id LIKE 'n/3d7fe7e4%';
-- Resultado: name="Bro", symbol="BRO", description="The memecoin of the UTXBros",
--            image_url="https://bro.charms.dev", decimals=8, total_supply=0
```

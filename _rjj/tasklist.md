# Tasklist: Integridad de Datos del Indexer

## 🔴 Problemas Detectados en Producción

### 1. **Assets: total_supply = 1 (CRÍTICO)**

- **Problema**: Todos los assets tienen `total_supply = 1` en lugar de la suma real de UTXOs
- **Causa**: El indexer crea el asset con supply=1 inicial y nunca lo actualiza
- **Solución**: Implementar actualización incremental de supply al procesar cada charm

### 2. **Stats Holders: charm_count negativo**

- **Problema**: Hay holders con `charm_count = -116` (valores negativos)
- **Causa**: Se restan charms gastados pero la lógica de conteo falla
- **Solución**: Revisar lógica de incremento/decremento en `stats_holders_service`

### 3. **Assets: Metadatos NULL**

- **Problema**: Muchos assets tienen `name`, `symbol`, `description`, `image_url` = NULL
- **Causa**: El indexer no extrae metadatos del NFT de referencia (n/HASH)
- **Solución**: Al crear token (t/HASH), buscar NFT (n/HASH) y copiar metadatos

---

## ✅ Tareas de Implementación en el Indexer

### **A. Supply Calculation (CRÍTICO)**

- [ ] Al crear nuevo charm → `UPDATE assets SET total_supply = total_supply + amount WHERE app_id = ?`
- [ ] Al marcar charm como spent → `UPDATE assets SET total_supply = total_supply - amount WHERE app_id = ?`
- [ ] Validar que supply nunca sea negativo
- [ ] Implementar en `indexer/src/domain/services/asset_supply_calculator.rs`

### **B. Stats Holders (CRÍTICO)**

- [ ] Al crear charm → `INSERT/UPDATE stats_holders SET charm_count = charm_count + 1, total_amount = total_amount + amount`
- [ ] Al marcar charm como spent → `UPDATE stats_holders SET charm_count = charm_count - 1, total_amount = total_amount - amount`
- [ ] Validar que charm_count nunca sea negativo (si es 0, DELETE row)
- [ ] Implementar en `indexer/src/infrastructure/persistence/repositories/stats_holders_repository.rs`

### **C. Asset Metadata Extraction**

- [ ] Al detectar token (t/HASH), extraer HASH
- [ ] Buscar NFT con app_id = n/HASH en tabla charms
- [ ] Extraer de NFT.data: `name`, `symbol`, `description`, `image`, `decimals`
- [ ] Guardar metadatos en tabla assets
- [ ] Implementar en `indexer/src/application/services/spell_detection.rs`

### **D. Decimals System**

- [ ] Implementar extracción de `decimals` del NFT de referencia
- [ ] Usar decimals para calcular supply correcto: `amount / 10^decimals`
- [ ] Default a 8 decimals si no existe NFT o campo
- [ ] Máximo 18 decimals (validación)
- [ ] Ver: `indexer/src/domain/models/asset_metadata.rs`

### **E. UTXO Tracking (Ya implementado, verificar)**

- [x] Campo `spent` en tabla charms
- [x] Campo `vout` en tabla charms
- [ ] Verificar que `mark_charms_as_spent_batch()` funciona correctamente
- [ ] Verificar que se actualiza supply al marcar como spent

---

## 🔧 Scripts de Corrección Manual (Documentar)

### Scripts en `/scripts` que arreglan datos:

1. **`regenerate_assets_from_charms.py`**: Recalcula supply sumando charms UNSPENT
2. **`populate_stats_holders.py`**: Regenera tabla stats_holders desde charms
3. **`analyze_bro_token_utxos.py`**: Analiza distribución de UTXOs del BRO token

### Acción:

- [ ] Mover archivos `.md` a `/scripts/documentation/`
- [ ] Mantener scripts `.py` y `.sh` en `/scripts/`
- [ ] Documentar qué hace cada script y cuándo usarlo
- [ ] Crear script maestro que ejecute todos en orden correcto

---

## 📊 Verificación de Integridad

### Queries de validación:

```sql
-- 1. Verificar supply correcto
SELECT app_id, total_supply,
       (SELECT SUM(amount) FROM charms WHERE charms.app_id = assets.app_id AND spent = false) as calculated_supply
FROM assets
WHERE total_supply != (SELECT SUM(amount) FROM charms WHERE charms.app_id = assets.app_id AND spent = false);

-- 2. Verificar holders sin negativos
SELECT * FROM stats_holders WHERE charm_count < 0 OR total_amount < 0;

-- 3. Verificar assets sin metadatos
SELECT COUNT(*) FROM assets WHERE name IS NULL AND asset_type IN ('token', 'nft');
```

---

## 🎯 Prioridad de Implementación

1. **CRÍTICO**: Supply calculation (A)
2. **CRÍTICO**: Stats holders fix (B)
3. **ALTO**: Asset metadata extraction (C)
4. **MEDIO**: Decimals system (D)
5. **BAJO**: Verificación UTXO tracking (E)

---

## 📝 Notas

- Los scripts manuales arreglan los datos ACTUALES pero no previenen el problema
- Hay que implementar la lógica correcta en el indexer para datos FUTUROS
- Después de implementar, ejecutar scripts de corrección una última vez
- Monitorear queries de validación periódicamente

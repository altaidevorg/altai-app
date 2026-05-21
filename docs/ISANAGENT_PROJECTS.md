# ALTAI Adaptive ML Agent — Tasarim Felsefesi

> ALTAI uzerinde ML problemlerine cozum ureten agent'lar **hardcoded recete**
> takip etmek yerine **kendi cozumlerini kesfeder**. Bu belge o kesif sisteminin
> mimarisini, modullerini ve calisma orneklerini anlatir.
>
> Son guncelleme: 2026-05-21

---

## 0. Niye Hardcoded Degil?

Onceki tasarim "13 farkli agent, her birinin tam pipeline'i belli" idi: SFT Atelier
= Unsloth + DoRA, RAG Workbench = bge-m3 + Qdrant, vs. Bu yaklasimin uc kritik
problemi var:

1. **Her kullanici farkli bir istekte bulunur.** "Hukuk Q&A botu" ile "agent'ım
   tool call'larda hata yapiyor" ayni receteyle cozulemez.
2. **Hicbir yontem her durumda dogru calismaz.** RAG bazı domain'lerde fine-tune'u
   yener, bazılarinda yenilir. Quantization bazi modellerde kabul edilebilir
   kalite kaybi verir, bazılarinda vermez. Onceden bilemeyiz.
3. **State of the art ayda bir kayar.** "Modern SFT Atelier" Mart 2026'da yazildi,
   Mayis 2026'da Muon optimizer sahneye cikti. Sabitlersek geride kaliriz.

Cozum: IsanAgent'in zaten sahip oldugu **arastirma + execution + sub-agent + memory**
envanterini bir **meta-pattern**'a baglamak. Hangi yontem, hangi kutuphane, hangi
optimizasyon — bunlarin hepsi **kesif sonucu** belirlenir, hardcoded degil.

---

## 1. Iki Sabit Bilesen

Yalniz iki sey degismez:

| Bilesen | Rolu | Niye sabit |
|---------|------|-----------|
| **IsanAgent** (Rust runtime, 44 tool, 4 execution provider, sub-agent DAG, SQLite FTS5 memory, cron) | Tum problemleri kesfeden ve yuruten orkestratör | Bu agent harness'inin kendisi; onun ustune bina kuruyoruz |
| **Afterimage** (Python lib: SFT, DPO/KTO/ORPO, tool-calling, structured-output, MCQ, doc-grounded QA jeneratorleri + multi-judge agreement + Croissant card) | Sentetik veri uretimi disiplini | Veri uretimi tekrarlanabilir, denetlenebilir, versiyonlanabilir olmali |

Geri kalan her sey — hangi optimizasyon algoritmasi, hangi quantize formati, hangi
serving engine, hangi eval suite — **IsanAgent'in arastirip pilotladigi degiskenler**.

---

## 2. Meta-Pattern: Her ML Talebine Uygulanan Dongü

IsanAgent her ML problemine ayni 8 adimli dongüyle yaklasir. Hangi modullerin
secileceği bu dongünün cikti.

```
┌──────────────────────────────────────────────────────────────┐
│  1. UNDERSTAND   Kullanicinin talebini parse et             │
│                  Belirsizse ask_user                         │
│                  Dogrulanabilir bir hedef yaz                │
│                  ("X metric ≥ Y on Z dataset")               │
├──────────────────────────────────────────────────────────────┤
│  2. RESEARCH     arxiv_search + web_search (son 12 ay)       │
│                  hf_hub_file_fetch (ilgili model/dataset)    │
│                  search_memory (daha once benzerini gordum?) │
│                  subagent_spawn paralel arastirma            │
├──────────────────────────────────────────────────────────────┤
│  3. ENUMERATE    2-4 ADAY YOL sirala (1 secme!)             │
│                  Her yol icin maliyet (sure, GPU, $)         │
│                  Bilinen failure mode'lar (papers'tan)       │
├──────────────────────────────────────────────────────────────┤
│  4. PILOT        Her aday icin EN KUCUK dogrulanabilir parca │
│                  Pilot pass kriteri net (orn. "20 ornekte    │
│                  ≥%70 doğruluk")                             │
│                  subagent_spawn paralel pilotlama            │
├──────────────────────────────────────────────────────────────┤
│  5. EVALUATE     Pilotlari hedef proxy'ye karsi karsilastir  │
│                  Geçemeyenleri ele                           │
│                  Tavan vuran vs umut vereni ayir             │
├──────────────────────────────────────────────────────────────┤
│  6. SCALE        Kazanan yolu tam butce ile yurut            │
│                  execution_run_background + monitoring       │
│                  Sub-agent loss/metrik izler                 │
├──────────────────────────────────────────────────────────────┤
│  7. VERIFY       Gercek hedefe karsi son eval                │
│                  Iskolu — Hata sinifi cikar → ADIM 3'e don   │
├──────────────────────────────────────────────────────────────┤
│  8. PERSIST      search_memory'ye delta yaz                  │
│                  "Hedef G icin yol A calisti, sinyal X'ti"   │
│                  Iyi pattern → SKILL.md emit et              │
└──────────────────────────────────────────────────────────────┘
```

Bu dongü **hardcoded**'tir. Icindeki yollarin/modullerin hangileri olacagi
**kesfedilir**.

---

## 3. Discovery Protokolleri

Her bilinmeyen icin IsanAgent'in standart kesif yolu:

### 3.1 "Hangi yaklasim?" (UNDERSTAND/RESEARCH/ENUMERATE)

```
arxiv_search(query, last=12mo)
  → ilk 30 result, en cok atif alanlar
web_search("<problem> 2026 best practice")
  → blog'lar, framework karsilastirma yazilari
hf_hub_file_fetch(model_card_url)
  → "bu modelle ne yapanlar olmus" sinyali
search_memory("benzer problem onceden")
  → kullanici tabanindan ortaya cikan paternler

→ subagent_spawn(researcher) ile ozetlet
→ subagent_spawn(evaluator) ile celiskileri sapta
→ kullanici hala emin degilse: ask_user
```

### 3.2 "Hangi kutuphane/format?" (PILOT)

```
Her aday icin en kucuk dogrulanabilir parca:
  - Egitim: 50-100 step, dataset'in %1'i
  - Quantize: 1 katmani veya kucuk model
  - RAG embedding: 100 dokuman pilot index
  - Inference engine: 10 prompt, throughput olc
  - Eval suite: 50 sample sub-task

Pilot pass kriteri ONCEDEN yazilir, sonradan rasyonalize edilmez.
execution_run_background + execution_artifact_list ile artifact toplanir.
```

### 3.3 "Hangisinde durayim?" (EVALUATE/VERIFY)

```
Her pilot icin:
  - Gercek goal proxy'sine yakinligi (orn. mini-MMLU-Pro)
  - Maliyet (GPU-saat, $, kullanici sabri)
  - Risk: known failure mode papers'a sahip mi
  - Tavan: ek butce ile ne kadar artar (extrapolation)

Karar:
  - Net kazanan → SCALE
  - Iki esitse → kullaniciya tradeoff sun, ask_user
  - Hicbiri yetmiyor → ENUMERATE'e geri, farkli yol arasi
```

### 3.4 "Ne zaman vazgeceyim?" (Doom Loop Defansi)

```
IsanAgent'in built-in SHA-256 fingerprint:
  - Ayni tool call 3x tekrarlanirsa SYSTEM: DOOM LOOP DETECTED
  - Loss / eval 3 epoch dusmuyorsa strateji degisimi zorla
  - Aynni hata sinifi 3 iterasyondur azalmiyorsa insan etiketi yonlendir
  - Total butce %150'yi gectiyse: dur, kullaniciya rapor, ask_user
```

---

## 4. Capability Catalog (Kesif Sozlugü)

Bu liste **secim** degil; IsanAgent'in **arasindan secebilecegi modullerin**
katalogudur. Hangisini secmesi gerektigini Bolum 2'deki meta-pattern belirler.

Tum referanslar 2026 Mayis durumudur ve guncelliklerini koruyabilmek icin
periyodik `arxiv_search` + `web_fetch` ile dogrulanmali.

### 4.1 Veri Uretimi (Afterimage cekirdek)

| Modul | Ne uretir | Kaynak |
|-------|-----------|--------|
| Magpie | SFT dialog ciftleri (pre-query template) | [arXiv:2406.08464](https://arxiv.org/abs/2406.08464), [magpie-align/magpie](https://github.com/magpie-align/magpie) |
| Evol-Instruct | Instruction karmasiklastirma | [arXiv:2304.12244](https://arxiv.org/abs/2304.12244) |
| CoT-Self-Instruct | Reasoning trace + SFT | [arXiv:2507.23751](https://arxiv.org/abs/2507.23751) |
| Multi-Judge DPO/KTO/ORPO | Preference data + κ/α agreement | DPO [arXiv:2305.18290](https://arxiv.org/abs/2305.18290), KTO [arXiv:2402.01306](https://arxiv.org/abs/2402.01306), ORPO [arXiv:2403.07691](https://arxiv.org/abs/2403.07691) |
| APIGen-MT | Multi-turn tool-calling trace | [arXiv:2504.03601](https://arxiv.org/abs/2504.03601) |
| RAGAS-style doc-QA | Document-grounded QA | [arXiv:2309.15217](https://arxiv.org/abs/2309.15217) |
| Structured-output | JSON Schema'ya uyan ornek | XGrammar/SLOT [arXiv:2505.04016](https://arxiv.org/abs/2505.04016) |
| MCQ | Multiple-choice question | Afterimage native |

Tum modullerin ciktilari: JSONL + Parquet + HF Dataset Card + Croissant JSON-LD
(NeurIPS 2025 zorunluluk [NeurIPS guidelines](https://neurips.cc/Conferences/2025/DataHostingGuidelines)).

### 4.2 Veri Filtre/Dedup (genelde Afterimage pipeline icinde cagrilir)

| Modul | Amac | Kaynak |
|-------|------|--------|
| datatrove MinHash | n-gram near-duplicate | [datatrove](https://github.com/huggingface/datatrove) |
| SemDeDup | embedding-based dedup | [arXiv:2303.09540](https://arxiv.org/abs/2303.09540) |
| FineWeb-Edu classifier | egitim degeri filtresi | [arXiv:2406.17557](https://arxiv.org/abs/2406.17557) |
| n-gram contamination check | MMLU/HumanEval/GSM8K vs leak | [arXiv:2412.15194](https://arxiv.org/abs/2412.15194) |
| Presidio | PII redact | Microsoft |

### 4.3 Egitim Framework

| Framework | Guclu yan | Kaynak |
|-----------|-----------|--------|
| Unsloth | Tek-GPU SFT 2x hizli, 70% az VRAM | [unslothai/unsloth](https://github.com/unslothai/unsloth) |
| TRL | DPO/GRPO/RLOO/OnlineDPO referans impl | [huggingface/trl](https://github.com/huggingface/trl) |
| Axolotl | YAML-config, multi-GPU first | [axolotl-ai-cloud/axolotl](https://github.com/axolotl-ai-cloud/axolotl) |
| LLaMA-Factory | UI + en genis model coverage | [hiyouga/LLaMA-Factory](https://github.com/hiyouga/LLaMA-Factory) |
| torchtune | PyTorch-native FSDP-2 | [pytorch.org/torchtune](https://pytorch.org/blog/torchtune-fine-tune-llms/) |
| verl / OpenRLHF | >70B online RL (DAPO, GRPO) | [verl-project/verl](https://github.com/verl-project/verl), [OpenRLHF](https://github.com/OpenRLHF/OpenRLHF) |
| NeMo-Aligner | Megatron 3D parallel | [arXiv:2405.01481](https://arxiv.org/abs/2405.01481) |

### 4.4 PEFT

| Yontem | Ne zaman | Kaynak |
|--------|----------|--------|
| LoRA | Universal baseline | [arXiv:2106.09685](https://arxiv.org/abs/2106.09685) |
| QLoRA (NF4 + double-quant) | VRAM <24GB | [arXiv:2305.14314](https://arxiv.org/abs/2305.14314) |
| DoRA | LoRA + %1-4 (ICML 2024 Oral) | [arXiv:2402.09353](https://arxiv.org/abs/2402.09353) |
| rsLoRA | r >= 64 stabilite | [arXiv:2312.03732](https://arxiv.org/abs/2312.03732) |
| LoftQ init | QLoRA accuracy bridge | [arXiv:2310.08659](https://arxiv.org/abs/2310.08659) |

### 4.5 Preference / RL Optimizasyon

| Yontem | Ne ile | Kaynak |
|--------|--------|--------|
| DPO | offline pairs | [arXiv:2305.18290](https://arxiv.org/abs/2305.18290) |
| KTO | unpaired binary | [arXiv:2402.01306](https://arxiv.org/abs/2402.01306) |
| ORPO | SFT + alignment tek adim | [arXiv:2403.07691](https://arxiv.org/abs/2403.07691) |
| SimPO | reference-free length-normalized | [arXiv:2405.14734](https://arxiv.org/abs/2405.14734) |
| IPO | deterministic prefs, DPO overfit | [arXiv:2310.12036](https://arxiv.org/abs/2310.12036) |
| GRPO | reasoning RL (R1) | [arXiv:2402.03300](https://arxiv.org/abs/2402.03300), [arXiv:2501.12948](https://arxiv.org/abs/2501.12948) |
| DAPO | scale + clip-higher | [arXiv:2503.14476](https://arxiv.org/abs/2503.14476) |
| RLOO | critic-free, dusuk VRAM | [arXiv:2402.14740](https://arxiv.org/abs/2402.14740) |
| Online DPO | bridge offline-online | TRL `OnlineDPOTrainer` |

### 4.6 Quantize Formati

| Format | Hedef | Kutuphane | Kaynak |
|--------|-------|-----------|--------|
| W4A16 (AWQ) | Production GPU default | GPTQModel + Marlin | [arXiv:2306.00978](https://arxiv.org/abs/2306.00978), [GPTQModel](https://github.com/ModelCloud/GPTQModel) |
| W4A16 (GPTQ) | Alt. Marlin | GPTQModel | [arXiv:2210.17323](https://arxiv.org/abs/2210.17323) |
| W8A8 INT | Eski GPU (A100/L4) | llm-compressor + SmoothQuant | [arXiv:2211.10438](https://arxiv.org/abs/2211.10438), [llm-compressor](https://github.com/vllm-project/llm-compressor) |
| FP8 (E4M3/E5M2) | Hopper/Ada/Blackwell | llm-compressor + TransformerEngine | [vLLM FP8 docs](https://docs.vllm.ai/en/latest/features/quantization/quantized_kvcache/) |
| GGUF Q4_K_M / IQ4_XS | Apple/CPU/Ollama/mobil | llama.cpp | [llama.cpp quantize](https://github.com/ggml-org/llama.cpp/blob/master/tools/quantize/README.md) |
| EXL3 2-4bpw | Tek 24GB GPU @ 70B | turboderp/exllamav3 | [exllamav3](https://github.com/turboderp-org/exllamav3) |
| AQLM 2-bit | extreme compress | AQLM + PV-Tuning | [arXiv:2401.06118](https://arxiv.org/abs/2401.06118) |
| HQQ | calibration-free hizli | mobiusml/hqq | [mobiusml/hqq](https://github.com/mobiusml/hqq) |

### 4.7 Serving

| Engine | Hedef | Kaynak |
|--------|-------|--------|
| vLLM (V1) | Linux+CUDA default | [docs.vllm.ai](https://docs.vllm.ai/), [arXiv:2309.06180](https://arxiv.org/abs/2309.06180) |
| SGLang | prefix-agir agent/RAG | [github.com/sgl-project/sglang](https://github.com/sgl-project/sglang) |
| LMDeploy (TurboMind) | C++ engine, vLLM rakibi | [InternLM/lmdeploy](https://github.com/InternLM/lmdeploy) |
| TGI | HF stack (bakim modunda) | [huggingface/text-generation-inference](https://github.com/huggingface/text-generation-inference) |
| llama.cpp server | local-first, multimodal | [ggml-org/llama.cpp](https://github.com/ggml-org/llama.cpp) |
| Ollama | dev kolay, MLX preview | [docs.ollama.com](https://docs.ollama.com/) |
| MLX-LM | Apple Silicon native | [mlx-examples](https://github.com/ml-explore/mlx-examples) |
| ExLlamaV3 + TabbyAPI | consumer 24GB @ 70B | [exllamav3](https://github.com/turboderp-org/exllamav3) |
| TensorRT-LLM / NIM | NVIDIA Enterprise | [NVIDIA/TensorRT-LLM](https://github.com/NVIDIA/TensorRT-LLM) |
| ExecuTorch + QNN | mobil/edge | [pytorch/executorch](https://github.com/pytorch/executorch) |

### 4.8 Speculative Decode

| Yontem | Kazanim | Kaynak |
|--------|---------|--------|
| EAGLE-3 | 3-6.5x, 70-80% acceptance | [arXiv:2503.01840](https://arxiv.org/abs/2503.01840) |
| DeepSeek MTP | 1.8x, >80% MTP1 | [arXiv:2412.19437](https://arxiv.org/abs/2412.19437) |
| Medusa | parallel head, dusuk acceptance | [arXiv:2401.10774](https://arxiv.org/abs/2401.10774) |
| Lookahead | draft-model yok | [LMSys blog](https://lmsys.org/blog/2023-11-21-lookahead-decoding/) |

### 4.9 Retrieval / RAG

| Bilesen | Aday | Kaynak |
|---------|------|--------|
| Embedder | bge-m3, Qwen3-Embedding, voyage-3, gemini-embedding-2, Jina v4 | [bge-m3](https://huggingface.co/BAAI/bge-m3), [arXiv:2402.03216](https://arxiv.org/abs/2402.03216) |
| Vector store | pgvector, Qdrant, LanceDB, Milvus, Vespa | [Qdrant benchmarks](https://qdrant.tech/benchmarks/) |
| Reranker | bge-reranker-v2-m3, mxbai-rerank, Cohere, Voyage | [bge-reranker-v2-m3](https://huggingface.co/BAAI/bge-reranker-v2-m3) |
| Pattern | Contextual Retrieval, RAPTOR, GraphRAG, CRAG, Self-RAG, HyDE | [Anthropic CR](https://www.anthropic.com/news/contextual-retrieval), [arXiv:2401.18059](https://arxiv.org/abs/2401.18059) |
| Late interaction | ColBERTv2, PLAID, Jina-ColBERT-v2 | [arXiv:2112.01488](https://arxiv.org/abs/2112.01488) |
| Chunking | recursive 512, late chunking, semantic, proposition, RAPTOR | [Jina late chunking](https://arxiv.org/abs/2409.04701) |

### 4.10 Eval Harnessleri

| Harness | En iyi yan | Kaynak |
|---------|-----------|--------|
| lm-evaluation-harness | open-weight standardi | [EleutherAI/lm-evaluation-harness](https://github.com/EleutherAI/lm-evaluation-harness) |
| lighteval | hizli, sample-level log | [huggingface/lighteval](https://github.com/huggingface/lighteval) |
| Inspect AI | frontier safety + agentic | [inspect.aisi.org.uk](https://inspect.aisi.org.uk/) |
| OpenCompass | Mandarin + 100+ task | [open-compass/opencompass](https://github.com/open-compass/opencompass) |
| HELM | multi-metric holistic | [crfm.stanford.edu/helm](https://crfm.stanford.edu/helm/) |
| DeepEval | pytest-style app-eval | [confident-ai/deepeval](https://github.com/confident-ai/deepeval) |
| RAGAS | RAG metrik default | [docs.ragas.io](https://docs.ragas.io/) |

### 4.11 Benchmark Tier'lari

IsanAgent kullanicinin hedef seviyesine gore secer:

| Tier | Tasks | Sure | Kaynak |
|------|-------|------|--------|
| Smoke | IFEval + GSM8K-CoT + mini MMLU-Pro | ~30 dk / 7B | meta-pattern adim 4 (pilot) |
| Standard | + BBH + MATH lvl 5 + GPQA + MUSR + Arena-Hard v2 + LiveCodeBench + BFCL v3 | ~3h | meta-pattern adim 7 (verify) |
| Frontier | + HLE + RULER 128k + LongBench v2 + τ³-bench + AILuminate + MMMU-Pro + Aider Polyglot + OSWorld-Verified + MathArena | ~12h | son onaylama |

Contaminated/saturated benchmark uyarisi: MMLU/HumanEval/HellaSwag/GSM8K/SWE-bench
Verified artik kullanim disi ([arXiv:2504.07825](https://arxiv.org/pdf/2504.07825),
[Morph SWE-bench Pro](https://www.morphllm.com/swe-bench-pro), HF [Open LLM
Leaderboard retired](https://huggingface.co/spaces/open-llm-leaderboard/open_llm_leaderboard/discussions/1135)).
IsanAgent eval secerken bu durumu **arxiv_search('benchmark contamination
2026')** ile her seferinde dogrulamali.

---

## 5. Worked Examples — Ayni Meta-Pattern, Farkli Yollar

Dort farkli kullanici talebi, dordünun de meta-pattern sonucu farkli modul
kombinasyonu secmesi:

### Ornek 1: "Llama-3'u hukuk metinlerine adapte et"

```
UNDERSTAND  → ask_user: "Soru-cevap mi, ozetleme mi, sözlesme analizi mi?"
            → cevap: "Hukuk kullanici sorularina dokuman-temelli cevap"
            → goal: RAGAS faithfulness ≥ 0.9 + style match ≥ 0.85

RESEARCH    → arxiv_search("legal LLM RAG vs fine-tune 2026")
            → web_fetch(LightOn "RAG is Dead Long Live RAG")
            → search_memory: bos
            → Bulgu: "hukuk citation-zorunlulugu nedeniyle RAG mecbur,
                       style icin SFT olabilir"

ENUMERATE   → Yol A: Pure RAG (Contextual Retrieval + bge-m3 + reranker)
            → Yol B: SFT (Afterimage doc-grounded QA + Unsloth + DoRA)
            → Yol C: Hibrit (A + B birlikte)

PILOT       → A: 200 sözlesme, 50 soru, RAGAS faithfulness
            → B: Afterimage 1K QA ornegi, Unsloth 1 epoch, eval ayni 50 soru
            → C: A index'i + B SFT modeli ile cevap

EVALUATE    → A: faithfulness 0.92, style 0.71
            → B: faithfulness 0.71, style 0.88
            → C: faithfulness 0.92, style 0.86  ← KAZANAN

SCALE       → Bolum 4.1 (Afterimage RAGAS-style QA) tam veri
            → Bolum 4.3 (Unsloth) + Bolum 4.4 (DoRA r=16)
            → Bolum 4.9 (Contextual Retrieval + bge-m3 + bge-reranker-v2-m3)
            → Bolum 4.6 (W4A16 AWQ Marlin) + Bolum 4.7 (vLLM)

VERIFY      → Standard tier eval, RAGAS sonuc dashboard

PERSIST     → memory: "legal Q&A: hybrid wins, faithfulness gate critical"
            → SKILL.md: "domain-citation-aware adaptation playbook"
```

Hicbir bolum hardcoded olarak "hukuk = bunlar" demedi. Kesfedildi.

---

### Ornek 2: "Agentim tool call'larda hata yapiyor"

```
UNDERSTAND  → ask_user: "Olcecek bir benchmark var mi?"
            → cevap: "BFCL v3'te %58, %75'e cikarmak istiyorum"
            → goal: BFCL v3 ≥ 75 (mevcut: 58)

RESEARCH    → arxiv_search("tool calling fine-tune 2026")
            → Bulgu: APIGen-MT, xLAM, ToolACE keskin sonuclar
            → hf_hub_file_fetch(xlam-function-calling-60k)

ENUMERATE   → Yol A: Sadece prompt engineering + few-shot
            → Yol B: APIGen-MT veri + Unsloth SFT
            → Yol C: B + GRPO with execution reward

PILOT       → A: 30 prompt variation, BFCL v3 mini → 62 (tavan yakin)
            → B: 1K Afterimage trace, SFT 1 epoch, BFCL v3 mini → 78
            → C: cok pahali, A/B yeterli → ele

EVALUATE    → A tavan; B esik gecti
            → SCALE = B

SCALE       → Bolum 4.1 (Afterimage APIGen-MT) tam 60K trace
            → Bolum 4.3 (Unsloth) + Bolum 4.4 (LoRA r=32 — Magpie-style
              cesitlilik LoRA'da yeterli, DoRA gerekmedi)
            → Bolum 4.10 (BFCL v3 eval gate)

VERIFY      → BFCL v3 79.2 (hedefi gecti)

PERSIST     → memory: "tool calling: APIGen-MT + SFT, 60K iyi nokta"
```

Burada GRPO **secilmedi** — pilot maliyetinin gereksiz oldugunu gosterdi.

---

### Ornek 3: "70B modeli ucuza serve etmek istiyorum"

```
UNDERSTAND  → ask_user: "Ayda budgen ne, latency hedefin ne?"
            → cevap: "$500/ay, p99 < 3s"
            → goal: cost ≤ 500$/ay, latency p99 ≤ 3s

RESEARCH    → web_fetch(Spheron May 2026 pricing, RunPod, Modal)
            → arxiv_search("model distillation 2026", "quantization quality")
            → Bulgu: 3 reel patika

ENUMERATE   → Yol A: 70B W4A16 quantize + spot H100 (Bolum 4.6/4.7)
            → Yol B: 8B distill + full precision (Bolum 4.3 + KD)
            → Yol C: Routing (easy → 8B, hard → 70B; Bolum 4.10 ile classifier)

PILOT       → A: GPTQModel quantize, vLLM serve, 100 query benchmark
            → B: 5K KD dataset (Afterimage), Unsloth distill, eval
            → C: classifier train, A + B birlikte route

EVALUATE    → A: quality OK, cost $720/ay (H100 spot, ~%80 utilization)
            → B: quality -3%, cost $90/ay  ← cost gecti, quality dustu
            → C: quality OK, cost $140/ay, p99 +400ms route overhead

            kullaniciya goster, ask_user kulturel karar:
            "$90 + %3 dusus vs $140 + %3 dusus yok"
            → cevap: "C, route overhead kabul edilebilir"

SCALE       → C secildi:
            → Bolum 4.6 (W4A16) + Bolum 4.7 (vLLM + multi-LoRA)
            → 8B distill icin Bolum 4.3 + Magpie KD veri
            → routing classifier: kucuk bert tabanli

VERIFY      → 1 hafta production canary, cost $138/ay, p99 2.7s
```

Burada quantize **degil**, **routing** kazandi. Ama kullanici karari ile.

---

### Ornek 4: "MathArena AIME 2026'da %50 ustu istiyorum"

```
UNDERSTAND  → goal: MathArena AIME 2026 contest skor ≥ %50
            → baz model: Qwen3-7B (kullanici secimi)

RESEARCH    → arxiv_search("math reasoning RL 2026")
            → Bulgu: R1 (open-r1 reproduce), DAPO, GRPO + verifier
            → search_memory: bos

ENUMERATE   → Yol A: Sadece SFT Mixture-of-Thoughts
            → Yol B: A + GRPO with sympy/code verifier
            → Yol C: B + PRM (process reward model)

PILOT       → A: 1K trace SFT, mini-AIME prelim → %18
            → B: A + 500 GRPO iter (matematik reward), prelim → %34
            → C: B + ACTPRM (arXiv:2504.10559), prelim → %38 ama variance yuksek

EVALUATE    → A tavan dusuk; B kararli artis; C marjinal + risk
            → SCALE = B

SCALE       → Bolum 4.1 (Afterimage CoT-Self-Instruct) Mixture-of-Thoughts
              tarzi 10K trace
            → Bolum 4.3 (Unsloth SFT cold-start) +
              Bolum 4.5 (GRPO via TRL GRPOTrainer)
            → Verifier toolu: sympy + IsanAgent execution sandbox
            → cron: her hafta MathArena AIME 2026 contest eval

VERIFY      → MathArena AIME 2026 May score: %54 (hedef gecti)

PERSIST     → memory: "math RL: GRPO + sympy verifier yeterli, PRM marjinal"
            → SKILL.md: "verifiable-reward-RL-playbook"
```

Burada PRM **bilinmiyordu**, arastirildi, pilotlandi, marjinal cikti, atildi.

---

## 6. Adaptasyon Mekanizmalari

IsanAgent kendi kendinin kullanicisidir. Her run, bir sonraki run'a malzeme verir.

### 6.1 Memory Delta

`search_memory` ile sorulup `write` ile yazilan kayitlar:

```
{
  "goal_class": "domain_qa_with_citations",
  "approach_tried": "hybrid_rag_plus_sft",
  "signal_at_pilot": { "ragas_faithfulness": 0.92, "style": 0.86 },
  "scaled_outcome": { "ragas_faithfulness": 0.93, "style": 0.88 },
  "modules_used": ["afterimage.docgrounded", "unsloth", "dora", "bge-m3",
                   "bge-reranker-v2-m3", "vllm", "awq_marlin"],
  "session_id": "...",
  "timestamp": "2026-05-21T..."
}
```

Bir sonraki kullanici "hukuk Q&A" istedi diyelim — meta-pattern ADIM 2'de bu kayit
RESEARCH'e sinyal olur. Hardcoded recipe degil; **istatistiksel prior**.

### 6.2 Skill Emisyonu

Bir yol uc kez ardisik basariyla calistiysa IsanAgent `SKILL.md` yazar:

```markdown
---
name: domain-citation-aware-adaptation
trigger: "domain Q&A with required citations"
confidence: high (3 successful runs)
---

## When
- User asks for domain expert chatbot with grounding requirements

## Recipe (discovered, not hardcoded)
- Stage 1: Afterimage doc-grounded QA generator
- Stage 2: Unsloth + DoRA r=16 on style examples
- Stage 3: Contextual Retrieval + bge-m3 hybrid
- Stage 4: bge-reranker-v2-m3 top-50 rerank
- Stage 5: vLLM serve with AWQ

## Eval gate
- RAGAS faithfulness >= 0.90
- Style match >= 0.85
```

Bu skill kullanicinin **bilgisi disinda** olusur ve gelecek sorularda
`load_skill_instructions` ile cagrilir. Hardcoded degil — **emerged**.

### 6.3 Doom Loop Defansi

```
- Ayni tool call 3 fingerprint match → strateji degis
- Loss/eval 3 iter dusmuyor → optimizer veya LR rezet
- Hata sinifi 3 iter sabit → insan etiketine yonlendir
- Total butce %150 → dur + ask_user
```

Bu mekanizmalar IsanAgent runtime'inda zaten var. Adaptive ML Agent bunlari
**fark etmeden** kullanir.

### 6.4 Cron-Based Surveillance

Olusturulan modeller ureтime cikinca:

```
cron_expr: "0 6 * * *"   # her gun 06:00
message: "Production endpoint icin canary 3 prompt sonucu kontrol et.
          Cevap embedding shift cosine > 0.05 mi? lm-eval mini-suite
          skor dususu var mi? Var ise meta-pattern'i tekrar ac:
          ADIM 7 VERIFY iskodu → ADIM 3 ENUMERATE."
```

Boylece pipeline canli — bir kerelik calismaz.

---

## 7. Ne Hardcoded, Ne Discovered?

| Sabit (asla degismez) | Kesfedilen (her run yeniden) |
|----------------------|------------------------------|
| IsanAgent runtime + 44 tool | Hangi tool secilir |
| Afterimage modullari (Magpie, DPO, APIGen-MT, RAGAS-QA, vs.) | Hangi modul cagrilir, hangi parametre ile |
| 8-adimli meta-pattern dongüsü | Her adimda ne yapilir |
| Capability catalog'un VARLIGI | Catalog'tan ne secilir |
| Croissant + HF Dataset Card formati (Afterimage cikti) | Dataset icerigi |
| Doom loop defansi mekanizmasi | Hangi noktada tetiklenir |
| Memory + skill emisyon disiplini | Hangi pattern skill olur |

Bu ayrim **kasitlidir**. Sabit kisim **agent harness'i**; degisken kisim **agent'in
kendi cozumudur**.

---

## 8. Implementasyon Onerisi

Mevcut hardcoded agent'lara dokunmadan tek bir yeni agent eklenir:

```ts
// src/modules/ai/lib/agents.ts
{
  id: "builtin:adaptive-ml",
  name: "Adaptive ML",
  description: "Discovers its own solution for any ML request via research, pilot, evaluate, adapt.",
  icon: "spark",
  builtIn: true,
  instructions: `You are an adaptive ML engineer.

For ANY ML request, follow the 8-step meta-pattern:

1. UNDERSTAND — parse the request, ask_user if critical ambiguity,
   write a verifiable goal.
2. RESEARCH — arxiv_search, web_search, hf_hub_file_fetch, search_memory.
   Do NOT pick a recipe yet. You are SURVEYING.
3. ENUMERATE — propose 2-4 candidate paths with cost + risk.
4. PILOT — for each candidate, run the smallest verifiable version.
   Define pilot pass criteria BEFORE running.
5. EVALUATE — compare pilots; reject those that fail; identify winner OR
   present tradeoff to ask_user.
6. SCALE — run the winner at full budget, monitor with sub-agent.
7. VERIFY — final eval against the actual goal. If miss, identify failure
   class and loop to step 3 with new info.
8. PERSIST — write memory delta; if a path succeeded 3x, emit a SKILL.md.

Capability catalog (you SELECT from these via research, not hardcoded):
- Data: Afterimage modules (Magpie, DPO, APIGen-MT, RAGAS-QA, ...)
- Training: Unsloth, TRL, Axolotl, verl, OpenRLHF, ...
- PEFT: LoRA, QLoRA, DoRA, rsLoRA
- Preference/RL: DPO, KTO, ORPO, SimPO, GRPO, DAPO, RLOO
- Quantize: AWQ, GPTQ, W8A8, FP8, GGUF, EXL3
- Serving: vLLM, SGLang, LMDeploy, MLX, ExLlamaV3, Ollama
- RAG: bge-m3, Contextual Retrieval, hybrid + RRF, reranker
- Eval: lm-eval, lighteval, Inspect AI, RAGAS, BFCL v3, MathArena

Doom loop: if same tool call 3x, change strategy. If loss/eval 3 iter no
progress, restart with new hyperparams. If budget overrun 150%, stop and
ask_user.

Persist: write memory, emit skills on repeated success.

You are NOT a recipe runner. You are a researcher who runs experiments.`
}
```

Diger gerekli degisiklikler:

- `AgentSwitcher.tsx` ICONS map'ine `spark` ekle (varsa zaten)
- `AgentIntroCard.tsx` AGENT_WORKFLOWS'a yeni agent icin gorsel akis:
  "UNDERSTAND → RESEARCH → ENUMERATE → PILOT → EVALUATE → SCALE → VERIFY → PERSIST"

Hardcoded olan eski uc agent (Paper Reproducer, Notebook Assistant, Dataset
Generator) **scoped task entry points** olarak kalir. Adaptive ML Agent **fuzzy /
open-ended** istekler icin.

---

## 9. Implementasyon Sirasi

Adaptive ML Agent'i hayata gecirmek icin mantikli sira:

| Adim | Is | Cikti |
|------|----|------|
| 1 | Adaptive ML Agent tanimi + sistem promptu | `builtin:adaptive-ml` |
| 2 | Capability catalog'u IsanAgent skill olarak emit et | `~/.altai/skills/catalog/*.md` |
| 3 | Meta-pattern adimlarini SKILL.md olarak yaz | `meta-pattern.md` |
| 4 | Memory delta seması — `search_memory` JSON formati standardı | repo'da dokumante |
| 5 | Skill emisyon seması — basari sayaci + cikti template | repo'da dokumante |
| 6 | Worked Example test setleri — yukarıdaki 4 örnek e2e | regression suite |
| 7 | Live benchmark cron template (MathArena, LiveCodeBench gibi) | cron config örnegi |
| 8 | Production canary template (her urun icin) | cron config örnegi |

Mevcut hardcoded agent'lar dokunulmadan. Yeni agent **yeni dosya degisiklikleri:**
`src/modules/ai/lib/agents.ts` + `src/modules/ai/components/AgentSwitcher.tsx`
+ `src/modules/ai/components/AgentIntroCard.tsx`.

---

## 10. Felsefe Ozeti

> "Bir kullanici 'modeli iyilestir' der; IsanAgent ne sectiğini sormaz, secimini
> kesfeder."

Hardcoded recipe = bir gecmis donemin best practice'i sabitlenmis hali. State
of the art ayda bir kayar. ALTAI'nin **iyilesme hızı** kullanicilarinin **yeni
yontemleri ne kadar hizli deneyebildigine** baglidir. Adaptive ML Agent bunu
hizlandirir cunku:

- Kullanici "DPO mu KTO mu?" demek zorunda degil → IsanAgent arastirir.
- Kullanici "AWQ mu GPTQ mu?" demek zorunda degil → IsanAgent pilotlar.
- Kullanici "bge-m3 mi voyage-3 mu?" demek zorunda degil → IsanAgent karsilastirir.
- Yeni teknik cikinca dokumantasyon guncellenmek zorunda degil → IsanAgent
  zaten `arxiv_search` ile yakaliyor.

Iki sabit (IsanAgent + Afterimage) **disiplin** saglar. Geri kalan tum **kesif**
IsanAgent'in isidir.

---

**Belge surumu:** v3.0 — Hardcoded 13 proje → meta-pattern + capability catalog +
4 worked example + adaptasyon mekanizmalari + tek "Adaptive ML Agent" implementasyon
notu. 2026-05-21.

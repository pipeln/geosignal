# Model licenses

GeoSignal's source code is MIT-licensed (see [LICENSE](LICENSE)). The optional GGUF model weights are **third-party works** and are **not** included in this repository, the Git history, or the packaged app. They are downloaded directly from Hugging Face only when a user asks for one in the Model lab.

| Preset | Upstream weights | License | Official terms |
| --- | --- | --- | --- |
| Qwen 2.5 1.5B | [Qwen/Qwen2.5-1.5B-Instruct-GGUF](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF) | Apache License 2.0 | [Qwen LICENSE](https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/blob/main/LICENSE) · [Apache-2.0](https://www.apache.org/licenses/LICENSE-2.0) |
| Llama 3.2 3B | [bartowski/Llama-3.2-3B-Instruct-GGUF](https://huggingface.co/bartowski/Llama-3.2-3B-Instruct-GGUF) | Meta Llama 3.2 Community License | [official terms](https://llama.meta.com/llama3_2/license) · [local snapshot](licenses/llama-3.2-community-license.txt) |
| Gemma 2 2B | [bartowski/gemma-2-2b-it-GGUF](https://huggingface.co/bartowski/gemma-2-2b-it-GGUF) | Gemma Terms of Use | [Gemma Terms of Use](https://ai.google.dev/gemma/terms) |

## What the app does

- Model lab shows the license **name** next to each preset.
- The app downloads the GGUF file from the upstream URL only after the user starts the download.
- The SHA-256 digest of each expected file is pinned in `src/ai.rs`. A downloaded file that does not match is deleted rather than loaded.
- GeoSignal does not redistribute, mirror, or sell model weights.

## Custom GGUF models

Custom models added in Model lab are also third-party weights. GeoSignal stores only the URL, filename, SHA-256 and license name supplied by the user, and downloads the file directly from that URL. It does not bundle or redistribute custom weights. The person adding a custom model is responsible for confirming the upstream license permits their use, including commercial use and any redistribution.

## Llama attribution

Llama 3.2 is licensed under the Llama 3.2 Community License, Copyright © Meta Platforms, Inc. All Rights Reserved. The license requires a prominent “Built with Llama” notice when Llama Materials, or a product or service containing them, are distributed or made available, and requires the Llama 3.2 notice to be retained in a `Notice` file with distributed copies.

GeoSignal does **not** bundle Llama weights. If you publish a fork or release that includes them, add the required “Built with Llama” notice and retain the license and Notice text. The complete upstream license is copied at [licenses/llama-3.2-community-license.txt](licenses/llama-3.2-community-license.txt).

## What users must do

Review and comply with the upstream terms before downloading or using a model, especially for commercial use, redistribution, or derivatives. The Llama and Gemma licenses are custom terms, not the MIT license used by GeoSignal. The Qwen preset is Apache-2.0; the full Apache-2.0 text also appears in [THIRD_PARTY_LICENSES.md](THIRD_PARTY_LICENSES.md).

If you publish a release or fork that **bundles** model weights, you are responsible for meeting the upstream license obligations, including any required notices, use restrictions, and distribution terms. Keeping weights out of the repository, as this project does, avoids implying that they are covered by GeoSignal's MIT license.

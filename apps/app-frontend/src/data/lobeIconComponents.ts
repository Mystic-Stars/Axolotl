import { defineAsyncComponent, type Component } from 'vue'

// Generated subset of @lobehub/icons-static-svg actually referenced by
// lobeProviderIcons / lobeModelIcons / AIIcon templates. Regenerate when
// those configs gain new slugs.

const icon_ace = () => import('@lobehub/icons-static-svg/icons/ace.svg?component')
const icon_ace_text = () => import('@lobehub/icons-static-svg/icons/ace-text.svg?component')
const icon_adobe = () => import('@lobehub/icons-static-svg/icons/adobe.svg?component')
const icon_adobe_color = () => import('@lobehub/icons-static-svg/icons/adobe-color.svg?component')
const icon_adobe_text = () => import('@lobehub/icons-static-svg/icons/adobe-text.svg?component')
const icon_ai2 = () => import('@lobehub/icons-static-svg/icons/ai2.svg?component')
const icon_ai2_color = () => import('@lobehub/icons-static-svg/icons/ai2-color.svg?component')
const icon_ai2_text = () => import('@lobehub/icons-static-svg/icons/ai2-text.svg?component')
const icon_ai21 = () => import('@lobehub/icons-static-svg/icons/ai21.svg?component')
const icon_ai21_brand_color = () => import('@lobehub/icons-static-svg/icons/ai21-brand-color.svg?component')
const icon_ai21_text = () => import('@lobehub/icons-static-svg/icons/ai21-text.svg?component')
const icon_ai302_color = () => import('@lobehub/icons-static-svg/icons/ai302-color.svg?component')
const icon_ai302_text = () => import('@lobehub/icons-static-svg/icons/ai302-text.svg?component')
const icon_ai360 = () => import('@lobehub/icons-static-svg/icons/ai360.svg?component')
const icon_ai360_color = () => import('@lobehub/icons-static-svg/icons/ai360-color.svg?component')
const icon_ai360_text = () => import('@lobehub/icons-static-svg/icons/ai360-text.svg?component')
const icon_aihubmix = () => import('@lobehub/icons-static-svg/icons/aihubmix.svg?component')
const icon_aihubmix_color = () => import('@lobehub/icons-static-svg/icons/aihubmix-color.svg?component')
const icon_aihubmix_text = () => import('@lobehub/icons-static-svg/icons/aihubmix-text.svg?component')
const icon_aimass = () => import('@lobehub/icons-static-svg/icons/aimass.svg?component')
const icon_aimass_color = () => import('@lobehub/icons-static-svg/icons/aimass-color.svg?component')
const icon_aimass_text = () => import('@lobehub/icons-static-svg/icons/aimass-text.svg?component')
const icon_aionlabs = () => import('@lobehub/icons-static-svg/icons/aionlabs.svg?component')
const icon_aionlabs_color = () => import('@lobehub/icons-static-svg/icons/aionlabs-color.svg?component')
const icon_aionlabs_text = () => import('@lobehub/icons-static-svg/icons/aionlabs-text.svg?component')
const icon_akashchat_color = () => import('@lobehub/icons-static-svg/icons/akashchat-color.svg?component')
const icon_akashchat_text = () => import('@lobehub/icons-static-svg/icons/akashchat-text.svg?component')
const icon_alibabacloud_color = () => import('@lobehub/icons-static-svg/icons/alibabacloud-color.svg?component')
const icon_alibabacloud_text_cn = () => import('@lobehub/icons-static-svg/icons/alibabacloud-text-cn.svg?component')
const icon_antgroup_text = () => import('@lobehub/icons-static-svg/icons/antgroup-text.svg?component')
const icon_anthropic = () => import('@lobehub/icons-static-svg/icons/anthropic.svg?component')
const icon_anthropic_text = () => import('@lobehub/icons-static-svg/icons/anthropic-text.svg?component')
const icon_arcee = () => import('@lobehub/icons-static-svg/icons/arcee.svg?component')
const icon_arcee_color = () => import('@lobehub/icons-static-svg/icons/arcee-color.svg?component')
const icon_arcee_text = () => import('@lobehub/icons-static-svg/icons/arcee-text.svg?component')
const icon_assemblyai = () => import('@lobehub/icons-static-svg/icons/assemblyai.svg?component')
const icon_assemblyai_color = () => import('@lobehub/icons-static-svg/icons/assemblyai-color.svg?component')
const icon_assemblyai_text = () => import('@lobehub/icons-static-svg/icons/assemblyai-text.svg?component')
const icon_aws = () => import('@lobehub/icons-static-svg/icons/aws.svg?component')
const icon_aws_color = () => import('@lobehub/icons-static-svg/icons/aws-color.svg?component')
const icon_aws_text = () => import('@lobehub/icons-static-svg/icons/aws-text.svg?component')
const icon_aya = () => import('@lobehub/icons-static-svg/icons/aya.svg?component')
const icon_aya_color = () => import('@lobehub/icons-static-svg/icons/aya-color.svg?component')
const icon_aya_text = () => import('@lobehub/icons-static-svg/icons/aya-text.svg?component')
const icon_azure_color = () => import('@lobehub/icons-static-svg/icons/azure-color.svg?component')
const icon_azure_text = () => import('@lobehub/icons-static-svg/icons/azure-text.svg?component')
const icon_azureai_color = () => import('@lobehub/icons-static-svg/icons/azureai-color.svg?component')
const icon_azureai_text = () => import('@lobehub/icons-static-svg/icons/azureai-text.svg?component')
const icon_baai = () => import('@lobehub/icons-static-svg/icons/baai.svg?component')
const icon_baai_text = () => import('@lobehub/icons-static-svg/icons/baai-text.svg?component')
const icon_baichuan = () => import('@lobehub/icons-static-svg/icons/baichuan.svg?component')
const icon_baichuan_color = () => import('@lobehub/icons-static-svg/icons/baichuan-color.svg?component')
const icon_baichuan_text = () => import('@lobehub/icons-static-svg/icons/baichuan-text.svg?component')
const icon_baiducloud = () => import('@lobehub/icons-static-svg/icons/baiducloud.svg?component')
const icon_baiducloud_color = () => import('@lobehub/icons-static-svg/icons/baiducloud-color.svg?component')
const icon_baiducloud_text = () => import('@lobehub/icons-static-svg/icons/baiducloud-text.svg?component')
const icon_bailian_color = () => import('@lobehub/icons-static-svg/icons/bailian-color.svg?component')
const icon_bailian_text = () => import('@lobehub/icons-static-svg/icons/bailian-text.svg?component')
const icon_bedrock_color = () => import('@lobehub/icons-static-svg/icons/bedrock-color.svg?component')
const icon_bedrock_text = () => import('@lobehub/icons-static-svg/icons/bedrock-text.svg?component')
const icon_bilibili = () => import('@lobehub/icons-static-svg/icons/bilibili.svg?component')
const icon_bilibili_color = () => import('@lobehub/icons-static-svg/icons/bilibili-color.svg?component')
const icon_bilibili_text = () => import('@lobehub/icons-static-svg/icons/bilibili-text.svg?component')
const icon_bilibiliindex = () => import('@lobehub/icons-static-svg/icons/bilibiliindex.svg?component')
const icon_bilibiliindex_text = () => import('@lobehub/icons-static-svg/icons/bilibiliindex-text.svg?component')
const icon_burncloud = () => import('@lobehub/icons-static-svg/icons/burncloud.svg?component')
const icon_burncloud_color = () => import('@lobehub/icons-static-svg/icons/burncloud-color.svg?component')
const icon_burncloud_text = () => import('@lobehub/icons-static-svg/icons/burncloud-text.svg?component')
const icon_bytedance = () => import('@lobehub/icons-static-svg/icons/bytedance.svg?component')
const icon_bytedance_color = () => import('@lobehub/icons-static-svg/icons/bytedance-color.svg?component')
const icon_bytedance_text = () => import('@lobehub/icons-static-svg/icons/bytedance-text.svg?component')
const icon_cerebras_brand_color = () => import('@lobehub/icons-static-svg/icons/cerebras-brand-color.svg?component')
const icon_chatglm = () => import('@lobehub/icons-static-svg/icons/chatglm.svg?component')
const icon_chatglm_color = () => import('@lobehub/icons-static-svg/icons/chatglm-color.svg?component')
const icon_chatglm_text = () => import('@lobehub/icons-static-svg/icons/chatglm-text.svg?component')
const icon_claude = () => import('@lobehub/icons-static-svg/icons/claude.svg?component')
const icon_claude_color = () => import('@lobehub/icons-static-svg/icons/claude-color.svg?component')
const icon_claude_text = () => import('@lobehub/icons-static-svg/icons/claude-text.svg?component')
const icon_cloudflare_color = () => import('@lobehub/icons-static-svg/icons/cloudflare-color.svg?component')
const icon_cloudflare_text = () => import('@lobehub/icons-static-svg/icons/cloudflare-text.svg?component')
const icon_codegeex = () => import('@lobehub/icons-static-svg/icons/codegeex.svg?component')
const icon_codegeex_color = () => import('@lobehub/icons-static-svg/icons/codegeex-color.svg?component')
const icon_codegeex_text = () => import('@lobehub/icons-static-svg/icons/codegeex-text.svg?component')
const icon_cogview = () => import('@lobehub/icons-static-svg/icons/cogview.svg?component')
const icon_cogview_color = () => import('@lobehub/icons-static-svg/icons/cogview-color.svg?component')
const icon_cogview_text = () => import('@lobehub/icons-static-svg/icons/cogview-text.svg?component')
const icon_cohere = () => import('@lobehub/icons-static-svg/icons/cohere.svg?component')
const icon_cohere_color = () => import('@lobehub/icons-static-svg/icons/cohere-color.svg?component')
const icon_cohere_text = () => import('@lobehub/icons-static-svg/icons/cohere-text.svg?component')
const icon_cometapi_color = () => import('@lobehub/icons-static-svg/icons/cometapi-color.svg?component')
const icon_cometapi_text = () => import('@lobehub/icons-static-svg/icons/cometapi-text.svg?component')
const icon_dalle = () => import('@lobehub/icons-static-svg/icons/dalle.svg?component')
const icon_dalle_color = () => import('@lobehub/icons-static-svg/icons/dalle-color.svg?component')
const icon_dalle_text = () => import('@lobehub/icons-static-svg/icons/dalle-text.svg?component')
const icon_dbrx = () => import('@lobehub/icons-static-svg/icons/dbrx.svg?component')
const icon_dbrx_color = () => import('@lobehub/icons-static-svg/icons/dbrx-color.svg?component')
const icon_dbrx_text = () => import('@lobehub/icons-static-svg/icons/dbrx-text.svg?component')
const icon_deepcogito = () => import('@lobehub/icons-static-svg/icons/deepcogito.svg?component')
const icon_deepcogito_color = () => import('@lobehub/icons-static-svg/icons/deepcogito-color.svg?component')
const icon_deepcogito_text = () => import('@lobehub/icons-static-svg/icons/deepcogito-text.svg?component')
const icon_deepmind = () => import('@lobehub/icons-static-svg/icons/deepmind.svg?component')
const icon_deepmind_color = () => import('@lobehub/icons-static-svg/icons/deepmind-color.svg?component')
const icon_deepmind_text = () => import('@lobehub/icons-static-svg/icons/deepmind-text.svg?component')
const icon_deepseek = () => import('@lobehub/icons-static-svg/icons/deepseek.svg?component')
const icon_deepseek_color = () => import('@lobehub/icons-static-svg/icons/deepseek-color.svg?component')
const icon_deepseek_text = () => import('@lobehub/icons-static-svg/icons/deepseek-text.svg?component')
const icon_dolphin = () => import('@lobehub/icons-static-svg/icons/dolphin.svg?component')
const icon_dolphin_text = () => import('@lobehub/icons-static-svg/icons/dolphin-text.svg?component')
const icon_doubao = () => import('@lobehub/icons-static-svg/icons/doubao.svg?component')
const icon_doubao_color = () => import('@lobehub/icons-static-svg/icons/doubao-color.svg?component')
const icon_doubao_text = () => import('@lobehub/icons-static-svg/icons/doubao-text.svg?component')
const icon_essentialai = () => import('@lobehub/icons-static-svg/icons/essentialai.svg?component')
const icon_essentialai_color = () => import('@lobehub/icons-static-svg/icons/essentialai-color.svg?component')
const icon_essentialai_text = () => import('@lobehub/icons-static-svg/icons/essentialai-text.svg?component')
const icon_fireworks = () => import('@lobehub/icons-static-svg/icons/fireworks.svg?component')
const icon_fireworks_color = () => import('@lobehub/icons-static-svg/icons/fireworks-color.svg?component')
const icon_fireworks_text = () => import('@lobehub/icons-static-svg/icons/fireworks-text.svg?component')
const icon_fishaudio = () => import('@lobehub/icons-static-svg/icons/fishaudio.svg?component')
const icon_fishaudio_text = () => import('@lobehub/icons-static-svg/icons/fishaudio-text.svg?component')
const icon_flux = () => import('@lobehub/icons-static-svg/icons/flux.svg?component')
const icon_flux_text = () => import('@lobehub/icons-static-svg/icons/flux-text.svg?component')
const icon_gemini = () => import('@lobehub/icons-static-svg/icons/gemini.svg?component')
const icon_gemini_color = () => import('@lobehub/icons-static-svg/icons/gemini-color.svg?component')
const icon_gemini_text = () => import('@lobehub/icons-static-svg/icons/gemini-text.svg?component')
const icon_gemma = () => import('@lobehub/icons-static-svg/icons/gemma.svg?component')
const icon_gemma_color = () => import('@lobehub/icons-static-svg/icons/gemma-color.svg?component')
const icon_gemma_text = () => import('@lobehub/icons-static-svg/icons/gemma-text.svg?component')
const icon_giteeai = () => import('@lobehub/icons-static-svg/icons/giteeai.svg?component')
const icon_giteeai_text = () => import('@lobehub/icons-static-svg/icons/giteeai-text.svg?component')
const icon_github = () => import('@lobehub/icons-static-svg/icons/github.svg?component')
const icon_github_text = () => import('@lobehub/icons-static-svg/icons/github-text.svg?component')
const icon_githubcopilot = () => import('@lobehub/icons-static-svg/icons/githubcopilot.svg?component')
const icon_githubcopilot_text = () => import('@lobehub/icons-static-svg/icons/githubcopilot-text.svg?component')
const icon_glmv = () => import('@lobehub/icons-static-svg/icons/glmv.svg?component')
const icon_glmv_color = () => import('@lobehub/icons-static-svg/icons/glmv-color.svg?component')
const icon_glmv_text = () => import('@lobehub/icons-static-svg/icons/glmv-text.svg?component')
const icon_google = () => import('@lobehub/icons-static-svg/icons/google.svg?component')
const icon_google_brand_color = () => import('@lobehub/icons-static-svg/icons/google-brand-color.svg?component')
const icon_google_color = () => import('@lobehub/icons-static-svg/icons/google-color.svg?component')
const icon_grok = () => import('@lobehub/icons-static-svg/icons/grok.svg?component')
const icon_grok_text = () => import('@lobehub/icons-static-svg/icons/grok-text.svg?component')
const icon_groq_text = () => import('@lobehub/icons-static-svg/icons/groq-text.svg?component')
const icon_huggingface_color = () => import('@lobehub/icons-static-svg/icons/huggingface-color.svg?component')
const icon_huggingface_text = () => import('@lobehub/icons-static-svg/icons/huggingface-text.svg?component')
const icon_hunyuan = () => import('@lobehub/icons-static-svg/icons/hunyuan.svg?component')
const icon_hunyuan_color = () => import('@lobehub/icons-static-svg/icons/hunyuan-color.svg?component')
const icon_hunyuan_text = () => import('@lobehub/icons-static-svg/icons/hunyuan-text.svg?component')
const icon_ibm = () => import('@lobehub/icons-static-svg/icons/ibm.svg?component')
const icon_ibm_text = () => import('@lobehub/icons-static-svg/icons/ibm-text.svg?component')
const icon_ideogram = () => import('@lobehub/icons-static-svg/icons/ideogram.svg?component')
const icon_ideogram_text = () => import('@lobehub/icons-static-svg/icons/ideogram-text.svg?component')
const icon_inception = () => import('@lobehub/icons-static-svg/icons/inception.svg?component')
const icon_inception_text = () => import('@lobehub/icons-static-svg/icons/inception-text.svg?component')
const icon_infinigence_color = () => import('@lobehub/icons-static-svg/icons/infinigence-color.svg?component')
const icon_infinigence_text_cn = () => import('@lobehub/icons-static-svg/icons/infinigence-text-cn.svg?component')
const icon_inflection = () => import('@lobehub/icons-static-svg/icons/inflection.svg?component')
const icon_inflection_text = () => import('@lobehub/icons-static-svg/icons/inflection-text.svg?component')
const icon_internlm = () => import('@lobehub/icons-static-svg/icons/internlm.svg?component')
const icon_internlm_color = () => import('@lobehub/icons-static-svg/icons/internlm-color.svg?component')
const icon_internlm_text = () => import('@lobehub/icons-static-svg/icons/internlm-text.svg?component')
const icon_jimeng = () => import('@lobehub/icons-static-svg/icons/jimeng.svg?component')
const icon_jimeng_color = () => import('@lobehub/icons-static-svg/icons/jimeng-color.svg?component')
const icon_jimeng_text = () => import('@lobehub/icons-static-svg/icons/jimeng-text.svg?component')
const icon_jina = () => import('@lobehub/icons-static-svg/icons/jina.svg?component')
const icon_jina_text = () => import('@lobehub/icons-static-svg/icons/jina-text.svg?component')
const icon_kling = () => import('@lobehub/icons-static-svg/icons/kling.svg?component')
const icon_kling_color = () => import('@lobehub/icons-static-svg/icons/kling-color.svg?component')
const icon_kling_text = () => import('@lobehub/icons-static-svg/icons/kling-text.svg?component')
const icon_kolors = () => import('@lobehub/icons-static-svg/icons/kolors.svg?component')
const icon_kolors_color = () => import('@lobehub/icons-static-svg/icons/kolors-color.svg?component')
const icon_kolors_text = () => import('@lobehub/icons-static-svg/icons/kolors-text.svg?component')
const icon_kwaipilot = () => import('@lobehub/icons-static-svg/icons/kwaipilot.svg?component')
const icon_kwaipilot_color = () => import('@lobehub/icons-static-svg/icons/kwaipilot-color.svg?component')
const icon_kwaipilot_text = () => import('@lobehub/icons-static-svg/icons/kwaipilot-text.svg?component')
const icon_lg = () => import('@lobehub/icons-static-svg/icons/lg.svg?component')
const icon_lg_color = () => import('@lobehub/icons-static-svg/icons/lg-color.svg?component')
const icon_lg_text = () => import('@lobehub/icons-static-svg/icons/lg-text.svg?component')
const icon_liquid = () => import('@lobehub/icons-static-svg/icons/liquid.svg?component')
const icon_liquid_text = () => import('@lobehub/icons-static-svg/icons/liquid-text.svg?component')
const icon_llava = () => import('@lobehub/icons-static-svg/icons/llava.svg?component')
const icon_llava_color = () => import('@lobehub/icons-static-svg/icons/llava-color.svg?component')
const icon_llava_text = () => import('@lobehub/icons-static-svg/icons/llava-text.svg?component')
const icon_lmstudio_text = () => import('@lobehub/icons-static-svg/icons/lmstudio-text.svg?component')
const icon_longcat = () => import('@lobehub/icons-static-svg/icons/longcat.svg?component')
const icon_longcat_color = () => import('@lobehub/icons-static-svg/icons/longcat-color.svg?component')
const icon_longcat_text = () => import('@lobehub/icons-static-svg/icons/longcat-text.svg?component')
const icon_menlo = () => import('@lobehub/icons-static-svg/icons/menlo.svg?component')
const icon_menlo_color = () => import('@lobehub/icons-static-svg/icons/menlo-color.svg?component')
const icon_menlo_text = () => import('@lobehub/icons-static-svg/icons/menlo-text.svg?component')
const icon_meta = () => import('@lobehub/icons-static-svg/icons/meta.svg?component')
const icon_meta_color = () => import('@lobehub/icons-static-svg/icons/meta-color.svg?component')
const icon_meta_text = () => import('@lobehub/icons-static-svg/icons/meta-text.svg?component')
const icon_microsoft = () => import('@lobehub/icons-static-svg/icons/microsoft.svg?component')
const icon_microsoft_color = () => import('@lobehub/icons-static-svg/icons/microsoft-color.svg?component')
const icon_microsoft_text = () => import('@lobehub/icons-static-svg/icons/microsoft-text.svg?component')
const icon_minimax = () => import('@lobehub/icons-static-svg/icons/minimax.svg?component')
const icon_minimax_color = () => import('@lobehub/icons-static-svg/icons/minimax-color.svg?component')
const icon_minimax_text = () => import('@lobehub/icons-static-svg/icons/minimax-text.svg?component')
const icon_mistral = () => import('@lobehub/icons-static-svg/icons/mistral.svg?component')
const icon_mistral_color = () => import('@lobehub/icons-static-svg/icons/mistral-color.svg?component')
const icon_mistral_text = () => import('@lobehub/icons-static-svg/icons/mistral-text.svg?component')
const icon_modelscope_color = () => import('@lobehub/icons-static-svg/icons/modelscope-color.svg?component')
const icon_modelscope_text = () => import('@lobehub/icons-static-svg/icons/modelscope-text.svg?component')
const icon_moonshot = () => import('@lobehub/icons-static-svg/icons/moonshot.svg?component')
const icon_moonshot_text = () => import('@lobehub/icons-static-svg/icons/moonshot-text.svg?component')
const icon_morph = () => import('@lobehub/icons-static-svg/icons/morph.svg?component')
const icon_morph_color = () => import('@lobehub/icons-static-svg/icons/morph-color.svg?component')
const icon_morph_text = () => import('@lobehub/icons-static-svg/icons/morph-text.svg?component')
const icon_nanobanana = () => import('@lobehub/icons-static-svg/icons/nanobanana.svg?component')
const icon_nanobanana_color = () => import('@lobehub/icons-static-svg/icons/nanobanana-color.svg?component')
const icon_nanobanana_text = () => import('@lobehub/icons-static-svg/icons/nanobanana-text.svg?component')
const icon_nebius_text = () => import('@lobehub/icons-static-svg/icons/nebius-text.svg?component')
const icon_newapi_color = () => import('@lobehub/icons-static-svg/icons/newapi-color.svg?component')
const icon_newapi_text = () => import('@lobehub/icons-static-svg/icons/newapi-text.svg?component')
const icon_nousresearch = () => import('@lobehub/icons-static-svg/icons/nousresearch.svg?component')
const icon_nousresearch_text = () => import('@lobehub/icons-static-svg/icons/nousresearch-text.svg?component')
const icon_nova = () => import('@lobehub/icons-static-svg/icons/nova.svg?component')
const icon_nova_color = () => import('@lobehub/icons-static-svg/icons/nova-color.svg?component')
const icon_nova_text = () => import('@lobehub/icons-static-svg/icons/nova-text.svg?component')
const icon_novita_color = () => import('@lobehub/icons-static-svg/icons/novita-color.svg?component')
const icon_novita_text = () => import('@lobehub/icons-static-svg/icons/novita-text.svg?component')
const icon_nvidia = () => import('@lobehub/icons-static-svg/icons/nvidia.svg?component')
const icon_nvidia_color = () => import('@lobehub/icons-static-svg/icons/nvidia-color.svg?component')
const icon_nvidia_text = () => import('@lobehub/icons-static-svg/icons/nvidia-text.svg?component')
const icon_ollama = () => import('@lobehub/icons-static-svg/icons/ollama.svg?component')
const icon_ollama_text = () => import('@lobehub/icons-static-svg/icons/ollama-text.svg?component')
const icon_openai = () => import('@lobehub/icons-static-svg/icons/openai.svg?component')
const icon_openai_text = () => import('@lobehub/icons-static-svg/icons/openai-text.svg?component')
const icon_openchat = () => import('@lobehub/icons-static-svg/icons/openchat.svg?component')
const icon_openchat_color = () => import('@lobehub/icons-static-svg/icons/openchat-color.svg?component')
const icon_openchat_text = () => import('@lobehub/icons-static-svg/icons/openchat-text.svg?component')
const icon_opencode_text = () => import('@lobehub/icons-static-svg/icons/opencode-text.svg?component')
const icon_openrouter = () => import('@lobehub/icons-static-svg/icons/openrouter.svg?component')
const icon_openrouter_color = () => import('@lobehub/icons-static-svg/icons/openrouter-color.svg?component')
const icon_openrouter_text = () => import('@lobehub/icons-static-svg/icons/openrouter-text.svg?component')
const icon_palm = () => import('@lobehub/icons-static-svg/icons/palm.svg?component')
const icon_palm_color = () => import('@lobehub/icons-static-svg/icons/palm-color.svg?component')
const icon_palm_text = () => import('@lobehub/icons-static-svg/icons/palm-text.svg?component')
const icon_perplexity = () => import('@lobehub/icons-static-svg/icons/perplexity.svg?component')
const icon_perplexity_color = () => import('@lobehub/icons-static-svg/icons/perplexity-color.svg?component')
const icon_perplexity_text = () => import('@lobehub/icons-static-svg/icons/perplexity-text.svg?component')
const icon_phind = () => import('@lobehub/icons-static-svg/icons/phind.svg?component')
const icon_phind_text = () => import('@lobehub/icons-static-svg/icons/phind-text.svg?component')
const icon_ppio_color = () => import('@lobehub/icons-static-svg/icons/ppio-color.svg?component')
const icon_ppio_text = () => import('@lobehub/icons-static-svg/icons/ppio-text.svg?component')
const icon_qiniu = () => import('@lobehub/icons-static-svg/icons/qiniu.svg?component')
const icon_qiniu_color = () => import('@lobehub/icons-static-svg/icons/qiniu-color.svg?component')
const icon_qiniu_text = () => import('@lobehub/icons-static-svg/icons/qiniu-text.svg?component')
const icon_qwen = () => import('@lobehub/icons-static-svg/icons/qwen.svg?component')
const icon_qwen_color = () => import('@lobehub/icons-static-svg/icons/qwen-color.svg?component')
const icon_qwen_text = () => import('@lobehub/icons-static-svg/icons/qwen-text.svg?component')
const icon_relace = () => import('@lobehub/icons-static-svg/icons/relace.svg?component')
const icon_relace_text = () => import('@lobehub/icons-static-svg/icons/relace-text.svg?component')
const icon_rwkv = () => import('@lobehub/icons-static-svg/icons/rwkv.svg?component')
const icon_rwkv_color = () => import('@lobehub/icons-static-svg/icons/rwkv-color.svg?component')
const icon_rwkv_text = () => import('@lobehub/icons-static-svg/icons/rwkv-text.svg?component')
const icon_sambanova_color = () => import('@lobehub/icons-static-svg/icons/sambanova-color.svg?component')
const icon_sambanova_text = () => import('@lobehub/icons-static-svg/icons/sambanova-text.svg?component')
const icon_search1api_color = () => import('@lobehub/icons-static-svg/icons/search1api-color.svg?component')
const icon_search1api_text = () => import('@lobehub/icons-static-svg/icons/search1api-text.svg?component')
const icon_sensenova = () => import('@lobehub/icons-static-svg/icons/sensenova.svg?component')
const icon_sensenova_color = () => import('@lobehub/icons-static-svg/icons/sensenova-color.svg?component')
const icon_sensenova_text = () => import('@lobehub/icons-static-svg/icons/sensenova-text.svg?component')
const icon_siliconcloud_color = () => import('@lobehub/icons-static-svg/icons/siliconcloud-color.svg?component')
const icon_siliconcloud_text = () => import('@lobehub/icons-static-svg/icons/siliconcloud-text.svg?component')
const icon_skywork = () => import('@lobehub/icons-static-svg/icons/skywork.svg?component')
const icon_skywork_color = () => import('@lobehub/icons-static-svg/icons/skywork-color.svg?component')
const icon_skywork_text = () => import('@lobehub/icons-static-svg/icons/skywork-text.svg?component')
const icon_sora = () => import('@lobehub/icons-static-svg/icons/sora.svg?component')
const icon_sora_color = () => import('@lobehub/icons-static-svg/icons/sora-color.svg?component')
const icon_sora_text = () => import('@lobehub/icons-static-svg/icons/sora-text.svg?component')
const icon_spark = () => import('@lobehub/icons-static-svg/icons/spark.svg?component')
const icon_spark_color = () => import('@lobehub/icons-static-svg/icons/spark-color.svg?component')
const icon_spark_text = () => import('@lobehub/icons-static-svg/icons/spark-text.svg?component')
const icon_stability = () => import('@lobehub/icons-static-svg/icons/stability.svg?component')
const icon_stability_color = () => import('@lobehub/icons-static-svg/icons/stability-color.svg?component')
const icon_stability_text = () => import('@lobehub/icons-static-svg/icons/stability-text.svg?component')
const icon_stepfun = () => import('@lobehub/icons-static-svg/icons/stepfun.svg?component')
const icon_stepfun_color = () => import('@lobehub/icons-static-svg/icons/stepfun-color.svg?component')
const icon_stepfun_text = () => import('@lobehub/icons-static-svg/icons/stepfun-text.svg?component')
const icon_straico_color = () => import('@lobehub/icons-static-svg/icons/straico-color.svg?component')
const icon_straico_text = () => import('@lobehub/icons-static-svg/icons/straico-text.svg?component')
const icon_streamlake_color = () => import('@lobehub/icons-static-svg/icons/streamlake-color.svg?component')
const icon_streamlake_text = () => import('@lobehub/icons-static-svg/icons/streamlake-text.svg?component')
const icon_suno = () => import('@lobehub/icons-static-svg/icons/suno.svg?component')
const icon_suno_text = () => import('@lobehub/icons-static-svg/icons/suno-text.svg?component')
const icon_tencentcloud_color = () => import('@lobehub/icons-static-svg/icons/tencentcloud-color.svg?component')
const icon_tencentcloud_text = () => import('@lobehub/icons-static-svg/icons/tencentcloud-text.svg?component')
const icon_tii = () => import('@lobehub/icons-static-svg/icons/tii.svg?component')
const icon_tii_color = () => import('@lobehub/icons-static-svg/icons/tii-color.svg?component')
const icon_tii_text = () => import('@lobehub/icons-static-svg/icons/tii-text.svg?component')
const icon_together_color = () => import('@lobehub/icons-static-svg/icons/together-color.svg?component')
const icon_together_text = () => import('@lobehub/icons-static-svg/icons/together-text.svg?component')
const icon_udio = () => import('@lobehub/icons-static-svg/icons/udio.svg?component')
const icon_udio_color = () => import('@lobehub/icons-static-svg/icons/udio-color.svg?component')
const icon_udio_text = () => import('@lobehub/icons-static-svg/icons/udio-text.svg?component')
const icon_upstage = () => import('@lobehub/icons-static-svg/icons/upstage.svg?component')
const icon_upstage_color = () => import('@lobehub/icons-static-svg/icons/upstage-color.svg?component')
const icon_upstage_text = () => import('@lobehub/icons-static-svg/icons/upstage-text.svg?component')
const icon_v0 = () => import('@lobehub/icons-static-svg/icons/v0.svg?component')
const icon_vercel = () => import('@lobehub/icons-static-svg/icons/vercel.svg?component')
const icon_vercel_text = () => import('@lobehub/icons-static-svg/icons/vercel-text.svg?component')
const icon_vertexai = () => import('@lobehub/icons-static-svg/icons/vertexai.svg?component')
const icon_vertexai_color = () => import('@lobehub/icons-static-svg/icons/vertexai-color.svg?component')
const icon_vertexai_text = () => import('@lobehub/icons-static-svg/icons/vertexai-text.svg?component')
const icon_vllm_color = () => import('@lobehub/icons-static-svg/icons/vllm-color.svg?component')
const icon_vllm_text = () => import('@lobehub/icons-static-svg/icons/vllm-text.svg?component')
const icon_volcengine_color = () => import('@lobehub/icons-static-svg/icons/volcengine-color.svg?component')
const icon_volcengine_text = () => import('@lobehub/icons-static-svg/icons/volcengine-text.svg?component')
const icon_voyage = () => import('@lobehub/icons-static-svg/icons/voyage.svg?component')
const icon_voyage_color = () => import('@lobehub/icons-static-svg/icons/voyage-color.svg?component')
const icon_voyage_text = () => import('@lobehub/icons-static-svg/icons/voyage-text.svg?component')
const icon_wenxin = () => import('@lobehub/icons-static-svg/icons/wenxin.svg?component')
const icon_wenxin_color = () => import('@lobehub/icons-static-svg/icons/wenxin-color.svg?component')
const icon_wenxin_text = () => import('@lobehub/icons-static-svg/icons/wenxin-text.svg?component')
const icon_workersai_color = () => import('@lobehub/icons-static-svg/icons/workersai-color.svg?component')
const icon_workersai_text = () => import('@lobehub/icons-static-svg/icons/workersai-text.svg?component')
const icon_xai_text = () => import('@lobehub/icons-static-svg/icons/xai-text.svg?component')
const icon_xiaomimimo = () => import('@lobehub/icons-static-svg/icons/xiaomimimo.svg?component')
const icon_xiaomimimo_text = () => import('@lobehub/icons-static-svg/icons/xiaomimimo-text.svg?component')
const icon_xinference_color = () => import('@lobehub/icons-static-svg/icons/xinference-color.svg?component')
const icon_xinference_text = () => import('@lobehub/icons-static-svg/icons/xinference-text.svg?component')
const icon_yi = () => import('@lobehub/icons-static-svg/icons/yi.svg?component')
const icon_yi_color = () => import('@lobehub/icons-static-svg/icons/yi-color.svg?component')
const icon_yi_text = () => import('@lobehub/icons-static-svg/icons/yi-text.svg?component')
const icon_zai = () => import('@lobehub/icons-static-svg/icons/zai.svg?component')
const icon_zai_text = () => import('@lobehub/icons-static-svg/icons/zai-text.svg?component')
const icon_zenmux = () => import('@lobehub/icons-static-svg/icons/zenmux.svg?component')
const icon_zenmux_text = () => import('@lobehub/icons-static-svg/icons/zenmux-text.svg?component')
const icon_zeroone_text = () => import('@lobehub/icons-static-svg/icons/zeroone-text.svg?component')
const icon_zhipu_color = () => import('@lobehub/icons-static-svg/icons/zhipu-color.svg?component')
const icon_zhipu_text = () => import('@lobehub/icons-static-svg/icons/zhipu-text.svg?component')

const loaders: Record<string, () => Promise<{ default: Component }>> = {
	'ace': icon_ace,
	'ace-text': icon_ace_text,
	'adobe': icon_adobe,
	'adobe-color': icon_adobe_color,
	'adobe-text': icon_adobe_text,
	'ai2': icon_ai2,
	'ai2-color': icon_ai2_color,
	'ai2-text': icon_ai2_text,
	'ai21': icon_ai21,
	'ai21-brand-color': icon_ai21_brand_color,
	'ai21-text': icon_ai21_text,
	'ai302-color': icon_ai302_color,
	'ai302-text': icon_ai302_text,
	'ai360': icon_ai360,
	'ai360-color': icon_ai360_color,
	'ai360-text': icon_ai360_text,
	'aihubmix': icon_aihubmix,
	'aihubmix-color': icon_aihubmix_color,
	'aihubmix-text': icon_aihubmix_text,
	'aimass': icon_aimass,
	'aimass-color': icon_aimass_color,
	'aimass-text': icon_aimass_text,
	'aionlabs': icon_aionlabs,
	'aionlabs-color': icon_aionlabs_color,
	'aionlabs-text': icon_aionlabs_text,
	'akashchat-color': icon_akashchat_color,
	'akashchat-text': icon_akashchat_text,
	'alibabacloud-color': icon_alibabacloud_color,
	'alibabacloud-text-cn': icon_alibabacloud_text_cn,
	'antgroup-text': icon_antgroup_text,
	'anthropic': icon_anthropic,
	'anthropic-text': icon_anthropic_text,
	'arcee': icon_arcee,
	'arcee-color': icon_arcee_color,
	'arcee-text': icon_arcee_text,
	'assemblyai': icon_assemblyai,
	'assemblyai-color': icon_assemblyai_color,
	'assemblyai-text': icon_assemblyai_text,
	'aws': icon_aws,
	'aws-color': icon_aws_color,
	'aws-text': icon_aws_text,
	'aya': icon_aya,
	'aya-color': icon_aya_color,
	'aya-text': icon_aya_text,
	'azure-color': icon_azure_color,
	'azure-text': icon_azure_text,
	'azureai-color': icon_azureai_color,
	'azureai-text': icon_azureai_text,
	'baai': icon_baai,
	'baai-text': icon_baai_text,
	'baichuan': icon_baichuan,
	'baichuan-color': icon_baichuan_color,
	'baichuan-text': icon_baichuan_text,
	'baiducloud': icon_baiducloud,
	'baiducloud-color': icon_baiducloud_color,
	'baiducloud-text': icon_baiducloud_text,
	'bailian-color': icon_bailian_color,
	'bailian-text': icon_bailian_text,
	'bedrock-color': icon_bedrock_color,
	'bedrock-text': icon_bedrock_text,
	'bilibili': icon_bilibili,
	'bilibili-color': icon_bilibili_color,
	'bilibili-text': icon_bilibili_text,
	'bilibiliindex': icon_bilibiliindex,
	'bilibiliindex-text': icon_bilibiliindex_text,
	'burncloud': icon_burncloud,
	'burncloud-color': icon_burncloud_color,
	'burncloud-text': icon_burncloud_text,
	'bytedance': icon_bytedance,
	'bytedance-color': icon_bytedance_color,
	'bytedance-text': icon_bytedance_text,
	'cerebras-brand-color': icon_cerebras_brand_color,
	'chatglm': icon_chatglm,
	'chatglm-color': icon_chatglm_color,
	'chatglm-text': icon_chatglm_text,
	'claude': icon_claude,
	'claude-color': icon_claude_color,
	'claude-text': icon_claude_text,
	'cloudflare-color': icon_cloudflare_color,
	'cloudflare-text': icon_cloudflare_text,
	'codegeex': icon_codegeex,
	'codegeex-color': icon_codegeex_color,
	'codegeex-text': icon_codegeex_text,
	'cogview': icon_cogview,
	'cogview-color': icon_cogview_color,
	'cogview-text': icon_cogview_text,
	'cohere': icon_cohere,
	'cohere-color': icon_cohere_color,
	'cohere-text': icon_cohere_text,
	'cometapi-color': icon_cometapi_color,
	'cometapi-text': icon_cometapi_text,
	'dalle': icon_dalle,
	'dalle-color': icon_dalle_color,
	'dalle-text': icon_dalle_text,
	'dbrx': icon_dbrx,
	'dbrx-color': icon_dbrx_color,
	'dbrx-text': icon_dbrx_text,
	'deepcogito': icon_deepcogito,
	'deepcogito-color': icon_deepcogito_color,
	'deepcogito-text': icon_deepcogito_text,
	'deepmind': icon_deepmind,
	'deepmind-color': icon_deepmind_color,
	'deepmind-text': icon_deepmind_text,
	'deepseek': icon_deepseek,
	'deepseek-color': icon_deepseek_color,
	'deepseek-text': icon_deepseek_text,
	'dolphin': icon_dolphin,
	'dolphin-text': icon_dolphin_text,
	'doubao': icon_doubao,
	'doubao-color': icon_doubao_color,
	'doubao-text': icon_doubao_text,
	'essentialai': icon_essentialai,
	'essentialai-color': icon_essentialai_color,
	'essentialai-text': icon_essentialai_text,
	'fireworks': icon_fireworks,
	'fireworks-color': icon_fireworks_color,
	'fireworks-text': icon_fireworks_text,
	'fishaudio': icon_fishaudio,
	'fishaudio-text': icon_fishaudio_text,
	'flux': icon_flux,
	'flux-text': icon_flux_text,
	'gemini': icon_gemini,
	'gemini-color': icon_gemini_color,
	'gemini-text': icon_gemini_text,
	'gemma': icon_gemma,
	'gemma-color': icon_gemma_color,
	'gemma-text': icon_gemma_text,
	'giteeai': icon_giteeai,
	'giteeai-text': icon_giteeai_text,
	'github': icon_github,
	'github-text': icon_github_text,
	'githubcopilot': icon_githubcopilot,
	'githubcopilot-text': icon_githubcopilot_text,
	'glmv': icon_glmv,
	'glmv-color': icon_glmv_color,
	'glmv-text': icon_glmv_text,
	'google': icon_google,
	'google-brand-color': icon_google_brand_color,
	'google-color': icon_google_color,
	'grok': icon_grok,
	'grok-text': icon_grok_text,
	'groq-text': icon_groq_text,
	'huggingface-color': icon_huggingface_color,
	'huggingface-text': icon_huggingface_text,
	'hunyuan': icon_hunyuan,
	'hunyuan-color': icon_hunyuan_color,
	'hunyuan-text': icon_hunyuan_text,
	'ibm': icon_ibm,
	'ibm-text': icon_ibm_text,
	'ideogram': icon_ideogram,
	'ideogram-text': icon_ideogram_text,
	'inception': icon_inception,
	'inception-text': icon_inception_text,
	'infinigence-color': icon_infinigence_color,
	'infinigence-text-cn': icon_infinigence_text_cn,
	'inflection': icon_inflection,
	'inflection-text': icon_inflection_text,
	'internlm': icon_internlm,
	'internlm-color': icon_internlm_color,
	'internlm-text': icon_internlm_text,
	'jimeng': icon_jimeng,
	'jimeng-color': icon_jimeng_color,
	'jimeng-text': icon_jimeng_text,
	'jina': icon_jina,
	'jina-text': icon_jina_text,
	'kling': icon_kling,
	'kling-color': icon_kling_color,
	'kling-text': icon_kling_text,
	'kolors': icon_kolors,
	'kolors-color': icon_kolors_color,
	'kolors-text': icon_kolors_text,
	'kwaipilot': icon_kwaipilot,
	'kwaipilot-color': icon_kwaipilot_color,
	'kwaipilot-text': icon_kwaipilot_text,
	'lg': icon_lg,
	'lg-color': icon_lg_color,
	'lg-text': icon_lg_text,
	'liquid': icon_liquid,
	'liquid-text': icon_liquid_text,
	'llava': icon_llava,
	'llava-color': icon_llava_color,
	'llava-text': icon_llava_text,
	'lmstudio-text': icon_lmstudio_text,
	'longcat': icon_longcat,
	'longcat-color': icon_longcat_color,
	'longcat-text': icon_longcat_text,
	'menlo': icon_menlo,
	'menlo-color': icon_menlo_color,
	'menlo-text': icon_menlo_text,
	'meta': icon_meta,
	'meta-color': icon_meta_color,
	'meta-text': icon_meta_text,
	'microsoft': icon_microsoft,
	'microsoft-color': icon_microsoft_color,
	'microsoft-text': icon_microsoft_text,
	'minimax': icon_minimax,
	'minimax-color': icon_minimax_color,
	'minimax-text': icon_minimax_text,
	'mistral': icon_mistral,
	'mistral-color': icon_mistral_color,
	'mistral-text': icon_mistral_text,
	'modelscope-color': icon_modelscope_color,
	'modelscope-text': icon_modelscope_text,
	'moonshot': icon_moonshot,
	'moonshot-text': icon_moonshot_text,
	'morph': icon_morph,
	'morph-color': icon_morph_color,
	'morph-text': icon_morph_text,
	'nanobanana': icon_nanobanana,
	'nanobanana-color': icon_nanobanana_color,
	'nanobanana-text': icon_nanobanana_text,
	'nebius-text': icon_nebius_text,
	'newapi-color': icon_newapi_color,
	'newapi-text': icon_newapi_text,
	'nousresearch': icon_nousresearch,
	'nousresearch-text': icon_nousresearch_text,
	'nova': icon_nova,
	'nova-color': icon_nova_color,
	'nova-text': icon_nova_text,
	'novita-color': icon_novita_color,
	'novita-text': icon_novita_text,
	'nvidia': icon_nvidia,
	'nvidia-color': icon_nvidia_color,
	'nvidia-text': icon_nvidia_text,
	'ollama': icon_ollama,
	'ollama-text': icon_ollama_text,
	'openai': icon_openai,
	'openai-text': icon_openai_text,
	'openchat': icon_openchat,
	'openchat-color': icon_openchat_color,
	'openchat-text': icon_openchat_text,
	'opencode-text': icon_opencode_text,
	'openrouter': icon_openrouter,
	'openrouter-color': icon_openrouter_color,
	'openrouter-text': icon_openrouter_text,
	'palm': icon_palm,
	'palm-color': icon_palm_color,
	'palm-text': icon_palm_text,
	'perplexity': icon_perplexity,
	'perplexity-color': icon_perplexity_color,
	'perplexity-text': icon_perplexity_text,
	'phind': icon_phind,
	'phind-text': icon_phind_text,
	'ppio-color': icon_ppio_color,
	'ppio-text': icon_ppio_text,
	'qiniu': icon_qiniu,
	'qiniu-color': icon_qiniu_color,
	'qiniu-text': icon_qiniu_text,
	'qwen': icon_qwen,
	'qwen-color': icon_qwen_color,
	'qwen-text': icon_qwen_text,
	'relace': icon_relace,
	'relace-text': icon_relace_text,
	'rwkv': icon_rwkv,
	'rwkv-color': icon_rwkv_color,
	'rwkv-text': icon_rwkv_text,
	'sambanova-color': icon_sambanova_color,
	'sambanova-text': icon_sambanova_text,
	'search1api-color': icon_search1api_color,
	'search1api-text': icon_search1api_text,
	'sensenova': icon_sensenova,
	'sensenova-color': icon_sensenova_color,
	'sensenova-text': icon_sensenova_text,
	'siliconcloud-color': icon_siliconcloud_color,
	'siliconcloud-text': icon_siliconcloud_text,
	'skywork': icon_skywork,
	'skywork-color': icon_skywork_color,
	'skywork-text': icon_skywork_text,
	'sora': icon_sora,
	'sora-color': icon_sora_color,
	'sora-text': icon_sora_text,
	'spark': icon_spark,
	'spark-color': icon_spark_color,
	'spark-text': icon_spark_text,
	'stability': icon_stability,
	'stability-color': icon_stability_color,
	'stability-text': icon_stability_text,
	'stepfun': icon_stepfun,
	'stepfun-color': icon_stepfun_color,
	'stepfun-text': icon_stepfun_text,
	'straico-color': icon_straico_color,
	'straico-text': icon_straico_text,
	'streamlake-color': icon_streamlake_color,
	'streamlake-text': icon_streamlake_text,
	'suno': icon_suno,
	'suno-text': icon_suno_text,
	'tencentcloud-color': icon_tencentcloud_color,
	'tencentcloud-text': icon_tencentcloud_text,
	'tii': icon_tii,
	'tii-color': icon_tii_color,
	'tii-text': icon_tii_text,
	'together-color': icon_together_color,
	'together-text': icon_together_text,
	'udio': icon_udio,
	'udio-color': icon_udio_color,
	'udio-text': icon_udio_text,
	'upstage': icon_upstage,
	'upstage-color': icon_upstage_color,
	'upstage-text': icon_upstage_text,
	'v0': icon_v0,
	'vercel': icon_vercel,
	'vercel-text': icon_vercel_text,
	'vertexai': icon_vertexai,
	'vertexai-color': icon_vertexai_color,
	'vertexai-text': icon_vertexai_text,
	'vllm-color': icon_vllm_color,
	'vllm-text': icon_vllm_text,
	'volcengine-color': icon_volcengine_color,
	'volcengine-text': icon_volcengine_text,
	'voyage': icon_voyage,
	'voyage-color': icon_voyage_color,
	'voyage-text': icon_voyage_text,
	'wenxin': icon_wenxin,
	'wenxin-color': icon_wenxin_color,
	'wenxin-text': icon_wenxin_text,
	'workersai-color': icon_workersai_color,
	'workersai-text': icon_workersai_text,
	'xai-text': icon_xai_text,
	'xiaomimimo': icon_xiaomimimo,
	'xiaomimimo-text': icon_xiaomimimo_text,
	'xinference-color': icon_xinference_color,
	'xinference-text': icon_xinference_text,
	'yi': icon_yi,
	'yi-color': icon_yi_color,
	'yi-text': icon_yi_text,
	'zai': icon_zai,
	'zai-text': icon_zai_text,
	'zenmux': icon_zenmux,
	'zenmux-text': icon_zenmux_text,
	'zeroone-text': icon_zeroone_text,
	'zhipu-color': icon_zhipu_color,
	'zhipu-text': icon_zhipu_text,
}

const cache = new Map<string, Component>()

export function getLobeIconComponent(slug: string): Component | undefined {
	const cached = cache.get(slug)
	if (cached) return cached

	const loader = loaders[slug]
	if (!loader) return undefined

	const component = defineAsyncComponent(async () => {
		const loaded = await loader()
		return loaded.default
	})
	cache.set(slug, component)
	return component
}

export function hasLobeIcon(slug: string): boolean {
	return slug in loaders
}

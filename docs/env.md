# API key & config

VoiceDictation stores its settings in `config.json` under the OS-specific app config directory.

## Azure OpenAI API key

Set the API key in the Settings UI (Azure → API key), then click Save. The key is written to `config.json`.

You can also edit the config file manually:
- `azure.apiKey`: Azure OpenAI API key
- `azure.endpoint`: `https://<resource>.openai.azure.com`
- `azure.deployment`: deployment name
- `azure.apiVersion`: API version

## Security note

The API key is stored on disk (plain text). Treat the config file as sensitive data and protect your user account accordingly.

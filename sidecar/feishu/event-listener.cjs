/**
 * Feishu WebSocket Event Listener
 *
 * Uses @larksuiteoapi/node-sdk WSClient to establish a long connection
 * and receive im.message.receive_v1 events.
 *
 * Usage: node event-listener.cjs <appId> <appSecret>
 *
 * Output: One JSON object per line to stdout for each received message:
 *   { "message_id": "...", "chat_id": "...", "chat_type": "p2p|group",
 *     "sender_id": "...", "msg_type": "...", "text": "...", "resources": [...] }
 *
 * Lifecycle signals (also JSON per line):
 *   { "type": "ready" }           — WebSocket connected
 *   { "type": "error", "msg": "..." } — Fatal error
 *   { "type": "debug", ... }      — Diagnostic info
 *   { "type": "sdk_log", "level": "...", "args": [...] } — SDK internal logs
 */

'use strict';

const appId = process.argv[2];
const appSecret = process.argv[3];

if (!appId || !appSecret) {
  const errorMsg = JSON.stringify({ type: 'error', msg: 'Usage: node event-listener.cjs <appId> <appSecret>' });
  process.stdout.write(errorMsg + '\n');
  process.exit(1);
}

function writeLine(obj) {
  try {
    process.stdout.write(JSON.stringify(obj) + '\n');
  } catch (_) {
    // stdout broken — parent process gone
    process.exit(0);
  }
}

// Intercept console methods BEFORE loading the SDK so all SDK logs
// go through our JSON protocol instead of raw text to stdout.
const origConsole = {
  log: console.log,
  info: console.info,
  warn: console.warn,
  error: console.error,
  debug: console.debug,
};

function safeStringify(val) {
  if (typeof val === 'string') return val;
  try {
    return JSON.stringify(val);
  } catch (_) {
    // Circular reference or other stringify error
    try { return String(val); } catch (__) { return '[unstringifiable]'; }
  }
}

function makeInterceptor(level) {
  return function (...args) {
    try {
      writeLine({
        type: 'sdk_log',
        level,
        args: args.map(safeStringify),
      });
    } catch (_) {
      // Silently ignore interceptor errors to avoid breaking SDK
    }
  };
}

console.log = makeInterceptor('log');
console.info = makeInterceptor('info');
console.warn = makeInterceptor('warn');
console.error = makeInterceptor('error');
console.debug = makeInterceptor('debug');

/**
 * Parse Feishu message content based on message type.
 * Returns { text, resources } or null to skip the message.
 *
 * @param {string} msgType - Message type (text, image, post, file, audio, video, media, sticker)
 * @param {string} contentStr - JSON string of message content
 * @returns {{ text: string, resources: Array<{ type: string, key: string, name: string }> } | null}
 */
function parseMessageContent(msgType, contentStr) {
  let content;
  try {
    content = JSON.parse(contentStr || '{}');
  } catch (_) {
    content = {};
  }

  let text = '';
  const resources = [];

  switch (msgType) {
    case 'text':
      text = content.text || '';
      break;

    case 'image':
      text = '[图片]';
      if (content.image_key) {
        resources.push({ type: 'image', key: content.image_key, name: 'image.png' });
      }
      break;

    case 'post': {
      // Rich text: convert to Markdown
      // Feishu post content can be localized (zh_cn, en_us, etc.) or flat
      let postContent = content;
      // If content has locale keys, pick zh_cn > en_us > first available
      if (!content.content && !content.title) {
        const locales = ['zh_cn', 'en_us'];
        for (const loc of locales) {
          if (content[loc]) { postContent = content[loc]; break; }
        }
        if (!postContent.content && !postContent.title) {
          const firstKey = Object.keys(content).find(k => content[k] && content[k].content);
          if (firstKey) postContent = content[firstKey];
        }
      }

      const title = postContent.title ? `# ${postContent.title}\n\n` : '';
      const lines = [];

      for (const paragraph of (postContent.content || [])) {
        let line = '';
        for (const elem of paragraph) {
          const style = elem.style;
          let fragment = '';

          switch (elem.tag) {
            case 'text':
              fragment = elem.text || '';
              break;
            case 'a':
              fragment = `[${elem.text || elem.href}](${elem.href})`;
              break;
            case 'at':
              fragment = `@${elem.user_name || elem.user_id || 'user'}`;
              break;
            case 'img':
              if (elem.image_key) {
                const imgName = `post_image_${resources.length}.png`;
                resources.push({ type: 'image', key: elem.image_key, name: imgName });
                fragment = `[${imgName}]`;
              }
              break;
            case 'media':
              if (elem.file_key) {
                resources.push({ type: 'file', key: elem.file_key, name: 'media' });
              }
              if (elem.image_key) {
                const coverName = `media_cover_${resources.length}.png`;
                resources.push({ type: 'image', key: elem.image_key, name: coverName });
              }
              fragment = '[媒体]';
              break;
            case 'emotion':
              fragment = elem.emoji_type ? `:${elem.emoji_type}:` : '';
              break;
            default:
              fragment = elem.text || '';
              break;
          }

          // Apply Markdown styles (bold, italic, strikethrough, underline)
          if (style && fragment) {
            if (style.bold) fragment = `**${fragment}**`;
            if (style.italic) fragment = `*${fragment}*`;
            if (style.strikethrough) fragment = `~~${fragment}~~`;
            if (style.underline) fragment = `<u>${fragment}</u>`;
          }

          line += fragment;
        }
        lines.push(line);
      }
      text = title + lines.join('\n');
      break;
    }

    case 'file':
      text = `[文件] ${content.file_name || 'unknown'}`;
      if (content.file_key) {
        resources.push({ type: 'file', key: content.file_key, name: content.file_name || 'file' });
      }
      break;

    case 'audio':
      text = '[音频消息]';
      if (content.file_key) {
        resources.push({ type: 'file', key: content.file_key, name: 'audio.opus' });
      }
      break;

    case 'video':
      text = '[视频消息]';
      if (content.file_key) {
        resources.push({ type: 'file', key: content.file_key, name: 'video.mp4' });
      }
      if (content.image_key) {
        resources.push({ type: 'image', key: content.image_key, name: 'video_cover.png' });
      }
      break;

    case 'media':
      text = '[媒体消息]';
      if (content.file_key) {
        resources.push({ type: 'file', key: content.file_key, name: 'media' });
      }
      if (content.image_key) {
        resources.push({ type: 'image', key: content.image_key, name: 'media_cover.png' });
      }
      break;

    case 'sticker':
      // Skip stickers — no analysis value
      return null;

    default:
      writeLine({ type: 'debug', msg: 'unsupported message type', msgType });
      return null;
  }

  return { text, resources };
}

async function main() {
  let Lark;
  try {
    Lark = require('@larksuiteoapi/node-sdk');
  } catch (e) {
    writeLine({ type: 'error', msg: 'Failed to load @larksuiteoapi/node-sdk: ' + e.message });
    process.exit(1);
  }

  const baseConfig = { appId, appSecret };

  // Single Lark client instance reused across bot-info resolution and reaction handler
  const client = new Lark.Client({ appId, appSecret, appType: Lark.AppType.SelfBuild });

  // Resolve bot's own open_id for @mention detection
  let botOpenId = '';
  try {
    const resp = await client.request({ method: 'GET', url: '/open-apis/bot/v3/info' });
    botOpenId = resp?.bot?.open_id || '';
    writeLine({ type: 'debug', msg: 'bot open_id resolved', botOpenId });
  } catch (e) {
    writeLine({ type: 'error', msg: 'Failed to get bot info: ' + e.message });
  }

  const eventDispatcher = new Lark.EventDispatcher({}).register({
    'im.message.receive_v1': async (data) => {
      try {
        const message = data.message;
        if (!message) {
          writeLine({ type: 'debug', msg: 'no message field in event data' });
          return;
        }

        const msgType = message.message_type;

        // Parse sender info
        const sender = data.sender;
        const senderType = sender && sender.sender_type;

        // Skip messages sent by the bot itself (sender_type === 'app')
        if (senderType === 'app') {
          writeLine({ type: 'debug', msg: 'skipping bot self message' });
          return;
        }

        const senderId = (sender && sender.sender_id && sender.sender_id.open_id) || 'unknown';

        // Parse message content based on type
        const parsed = parseMessageContent(msgType, message.content);
        if (!parsed) {
          writeLine({ type: 'debug', msg: 'skipping unsupported message type', msgType });
          return;
        }

        let { text } = parsed;
        const { resources } = parsed;

        // Strip @bot mentions from text
        if (text) {
          text = text.replace(/@_user_\d+/g, '').trim();
        }

        // Skip if no text and no resources
        if (!text && resources.length === 0) {
          writeLine({ type: 'debug', msg: 'empty content after parse', msgType, rawContent: message.content });
          return;
        }

        // Detect if this bot was @mentioned
        const mentions = message.mentions || [];
        const mentionedBot = botOpenId
          ? mentions.some(m => m?.id?.open_id === botOpenId)
          : false;

        writeLine({
          message_id: message.message_id || '',
          chat_id: message.chat_id || '',
          chat_type: message.chat_type || '',
          event_source: 'message',
          sender_id: senderId,
          msg_type: msgType,
          text: text || '',
          resources,
          mentioned_bot: mentionedBot,
        });
      } catch (e) {
        writeLine({ type: 'error', msg: 'Event handler error: ' + e.message });
      }
    },

    // When someone adds a "Status_PrivateMessage" reaction to a group message,
    // treat the original message as a new request to the bot.
    'im.message.reaction.created_v1': async (data) => {
      try {
        const event = data.event || data;
        const emojiType = event.reaction_type?.emoji_type;
        const operatorType = event.operator_type;
        const messageId = event.message_id;

        // Only handle "Status_PrivateMessage" reactions; ignore reactions added by the bot itself
        if (emojiType !== 'Status_PrivateMessage' || operatorType === 'app') return;
        if (!messageId) return;

        // Fetch the original message content
        const msgResp = await client.im.v1.message.get({ path: { message_id: messageId } });
        const item = msgResp?.data?.items?.[0];
        if (!item) {
          writeLine({ type: 'debug', msg: 'reaction: failed to fetch message', messageId });
          return;
        }

        // Only handle group messages
        if (item.chat_type !== 'group') return;

        const msgType = item.msg_type;
        const chatId = item.chat_id || '';
        const senderId = item.sender?.id?.open_id || 'unknown';

        const parsed = parseMessageContent(msgType, item.body?.content);
        if (!parsed || (!parsed.text && parsed.resources.length === 0)) {
          writeLine({ type: 'debug', msg: 'reaction: empty or unsupported message content', msgType });
          return;
        }

        writeLine({
          message_id: messageId,
          chat_id: chatId,
          chat_type: 'group',
          event_source: 'reaction_replay',
          sender_id: senderId,
          msg_type: msgType,
          text: parsed.text || '',
          resources: parsed.resources,
          mentioned_bot: true, // bypass group mention filter — reaction is explicit intent
        });
      } catch (e) {
        writeLine({ type: 'error', msg: 'reaction handler error: ' + e.message });
      }
    },
  });

  const wsClient = new Lark.WSClient({
    ...baseConfig,
    loggerLevel: Lark.LoggerLevel.error,
  });

  try {
    await wsClient.start({ eventDispatcher });
    writeLine({ type: 'ready' });
  } catch (e) {
    writeLine({ type: 'error', msg: 'WSClient start failed: ' + e.message });
    process.exit(1);
  }
}

// Keep the process alive; handle termination gracefully
process.on('SIGTERM', () => process.exit(0));
process.on('SIGINT', () => process.exit(0));
// If parent dies (pipe broken), exit
process.stdout.on('error', () => process.exit(0));

main().catch((e) => {
  writeLine({ type: 'error', msg: 'Fatal: ' + e.message });
  process.exit(1);
});

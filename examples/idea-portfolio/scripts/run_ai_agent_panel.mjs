#!/usr/bin/env node

import fs from "node:fs";
import os from "node:os";
import path from "node:path";

const DEFAULT_BASE_URL = "http://127.0.0.1:8317/v1";
const DEFAULT_MODEL = "gpt-5.4";

const args = parseArgs(process.argv.slice(2));
const resultPath =
  args.result ?? "examples/idea-portfolio/results/consumer-ai-viral-50-source-weighted-result.json";
const portfolioPath =
  args.portfolio ?? "examples/idea-portfolio/results/consumer-ai-viral-50-source-weighted-portfolio.json";
const outputPath =
  args.output ?? "examples/idea-portfolio/results/consumer-ai-viral-50-ai-agent-panel.json";
const markdownPath =
  args.markdown ?? outputPath.replace(/\.json$/i, ".md");
const model = args.model ?? DEFAULT_MODEL;
const baseUrl = (args.baseUrl ?? process.env.CLIPROXYAPI_BASE_URL ?? DEFAULT_BASE_URL).replace(
  /\/$/,
  "",
);
const ideaLimit = parseInt(args.ideaLimit ?? "10", 10);
const personasCatalog = getPersonas();
const personaLimit = parseInt(args.personas ?? `${personasCatalog.length}`, 10);
const reasoningEffort = args.reasoningEffort ?? "medium";

const apiKey = readProxyApiKey();
const result = JSON.parse(fs.readFileSync(resultPath, "utf8"));
const portfolio = JSON.parse(fs.readFileSync(portfolioPath, "utf8"));
const ideasById = new Map(portfolio.ideas.map((idea) => [idea.id, idea]));
const ideas = result.ranked_ideas
  .slice(0, ideaLimit)
  .map((ranked, index) => ({
    rank: index + 1,
    idea_id: ranked.idea_id,
    idea_name: ranked.idea_name,
    simulator_score: ranked.overall_score,
    summary: ideasById.get(ranked.idea_id)?.summary ?? "",
  }));

if (ideas.length === 0) {
  throw new Error("No ideas found in result file.");
}

const selectedPersonas = personasCatalog.slice(0, personaLimit);
const agentRuns = [];

for (const [index, persona] of selectedPersonas.entries()) {
  console.error(
    `AI persona ${index + 1}/${selectedPersonas.length}: ${persona.name} (${persona.segment})`,
  );
  const response = await runPersona({ persona, ideas, model, baseUrl, apiKey, reasoningEffort });
  agentRuns.push(response);
}

const aggregate = aggregatePanel(agentRuns, ideas);
const artifact = {
  kind: "ai_agent_persona_panel",
  created_at: new Date().toISOString(),
  model,
  base_url: baseUrl,
  reasoning_effort: reasoningEffort,
  source_result: resultPath,
  source_portfolio: portfolioPath,
  note:
    "This is not the deterministic simulator. Each panel row is an LLM call instructed to respond as one synthetic person/persona; aggregation is computed after those responses.",
  ideas,
  personas: selectedPersonas,
  agent_runs: agentRuns,
  aggregate,
};

fs.mkdirSync(path.dirname(outputPath), { recursive: true });
fs.writeFileSync(outputPath, JSON.stringify(artifact, null, 2) + "\n");
fs.writeFileSync(markdownPath, renderMarkdown(artifact));
console.error(`Wrote ${outputPath}`);
console.error(`Wrote ${markdownPath}`);

function parseArgs(argv) {
  const parsed = {};
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    const next = argv[index + 1];
    if (!next || next.startsWith("--")) {
      parsed[key] = "true";
    } else {
      parsed[key] = next;
      index += 1;
    }
  }
  return parsed;
}

function readProxyApiKey() {
  if (process.env.CLIPROXYAPI_API_KEY) return process.env.CLIPROXYAPI_API_KEY;
  if (process.env.OPENAI_API_KEY) return process.env.OPENAI_API_KEY;
  const authPath = path.join(os.homedir(), ".codex", "auth.proxy.json");
  const raw = fs.readFileSync(authPath, "utf8");
  const parsed = JSON.parse(raw);
  if (!parsed.OPENAI_API_KEY) {
    throw new Error(`${authPath} did not include OPENAI_API_KEY.`);
  }
  return parsed.OPENAI_API_KEY;
}

async function runPersona({ persona, ideas, model, baseUrl, apiKey, reasoningEffort }) {
  const system = [
    "You are not a market analyst.",
    "You are simulating one realistic consumer/persona in a qualitative research panel.",
    "Stay in first person for quotes and objections.",
    "Do not average the market. Do not rank as a consultant. React as this person with their constraints, taste, skepticism, budget, social graph, and phone habits.",
    "Return valid JSON only. No markdown fences.",
  ].join(" ");
  const user = JSON.stringify(
    {
      task:
        "Evaluate these consumer AI app ideas as this one person. Pick what you would actually try, share, pay for, and keep using. Be specific and a little skeptical.",
      persona,
      ideas,
      output_schema: {
        persona_id: persona.id,
        first_person_summary: "2-4 sentences in this persona's own voice",
        ranked_reactions: [
          {
            idea_id: "exact idea id",
            rank: 1,
            try_intent: "0-100 integer",
            share_intent: "0-100 integer",
            pay_intent: "0-100 integer",
            retention_intent: "0-100 integer",
            why_it_hits: "specific reason in first person",
            biggest_objection: "specific objection in first person",
            trigger_to_try: "what would make this person try it this week",
            price_ceiling: "plain English price reaction",
            quote: "one sentence this person might say to a friend",
          },
        ],
        ignored_or_rejected: [
          {
            idea_id: "exact idea id",
            reason: "why this person would ignore it",
          },
        ],
        most_viral_artifact: "the output this person would most likely send/post",
        trust_breaker: "what would make them delete or distrust the app",
        surprise: "one non-obvious insight from this person's reaction",
      },
    },
    null,
    2,
  );

  const body = {
    model,
    stream: true,
    input: [
      { role: "system", content: [{ type: "input_text", text: system }] },
      { role: "user", content: [{ type: "input_text", text: user }] },
    ],
    reasoning: { effort: reasoningEffort },
    max_output_tokens: 2200,
  };

  const response = await fetch(`${baseUrl}/responses`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${apiKey}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!response.ok) {
    throw new Error(`LLM call failed: ${response.status} ${await response.text()}`);
  }
  const raw = await response.text();
  const text = parseSseText(raw);
  const parsed = parseJsonText(text);
  return {
    persona_id: persona.id,
    persona_name: persona.name,
    segment: persona.segment,
    raw_text: text,
    parsed,
  };
}

function parseSseText(raw) {
  const deltas = [];
  const dones = [];
  for (const line of raw.split(/\r?\n/)) {
    if (!line.startsWith("data: ")) continue;
    const payload = line.slice(6).trim();
    if (!payload || payload === "[DONE]") continue;
    let event;
    try {
      event = JSON.parse(payload);
    } catch {
      continue;
    }
    if (event.type === "response.output_text.delta") deltas.push(event.delta ?? "");
    if (event.type === "response.output_text.done") dones.push(event.text ?? "");
  }
  return (dones.join("\n") || deltas.join("")).trim();
}

function parseJsonText(text) {
  const cleaned = text
    .replace(/^```json\s*/i, "")
    .replace(/^```\s*/i, "")
    .replace(/```\s*$/i, "")
    .trim();
  try {
    return JSON.parse(cleaned);
  } catch {
    const first = cleaned.indexOf("{");
    const last = cleaned.lastIndexOf("}");
    if (first >= 0 && last > first) {
      return JSON.parse(cleaned.slice(first, last + 1));
    }
    throw new Error(`Could not parse JSON model output: ${text.slice(0, 500)}`);
  }
}

function aggregatePanel(agentRuns, ideas) {
  const byIdea = new Map(
    ideas.map((idea) => [
      idea.idea_id,
      {
        idea_id: idea.idea_id,
        idea_name: idea.idea_name,
        simulator_score: idea.simulator_score,
        mentions: 0,
        first_place_votes: 0,
        try_intent_sum: 0,
        share_intent_sum: 0,
        pay_intent_sum: 0,
        retention_intent_sum: 0,
        quotes: [],
        objections: [],
        triggers: [],
      },
    ]),
  );

  for (const run of agentRuns) {
    const reactions = run.parsed?.ranked_reactions ?? [];
    for (const reaction of reactions) {
      const row = byIdea.get(reaction.idea_id);
      if (!row) continue;
      row.mentions += 1;
      if (Number(reaction.rank) === 1) row.first_place_votes += 1;
      row.try_intent_sum += numberish(reaction.try_intent);
      row.share_intent_sum += numberish(reaction.share_intent);
      row.pay_intent_sum += numberish(reaction.pay_intent);
      row.retention_intent_sum += numberish(reaction.retention_intent);
      if (reaction.quote) row.quotes.push(`${run.persona_name}: ${reaction.quote}`);
      if (reaction.biggest_objection) {
        row.objections.push(`${run.persona_name}: ${reaction.biggest_objection}`);
      }
      if (reaction.trigger_to_try) {
        row.triggers.push(`${run.persona_name}: ${reaction.trigger_to_try}`);
      }
    }
  }

  const ranked = [...byIdea.values()]
    .map((row) => {
      const denom = Math.max(1, row.mentions);
      const tryIntent = Math.round(row.try_intent_sum / denom);
      const shareIntent = Math.round(row.share_intent_sum / denom);
      const payIntent = Math.round(row.pay_intent_sum / denom);
      const retentionIntent = Math.round(row.retention_intent_sum / denom);
      return {
        ...row,
        avg_try_intent: tryIntent,
        avg_share_intent: shareIntent,
        avg_pay_intent: payIntent,
        avg_retention_intent: retentionIntent,
        ai_panel_score: Math.round(
          tryIntent * 0.3 +
            shareIntent * 0.25 +
            payIntent * 0.2 +
            retentionIntent * 0.2 +
            row.first_place_votes * 2.5,
        ),
        quotes: row.quotes.slice(0, 5),
        objections: row.objections.slice(0, 5),
        triggers: row.triggers.slice(0, 5),
      };
    })
    .sort((left, right) => {
      return (
        right.ai_panel_score - left.ai_panel_score ||
        right.first_place_votes - left.first_place_votes ||
        right.mentions - left.mentions ||
        left.idea_id.localeCompare(right.idea_id)
      );
    });

  return {
    ranked,
    persona_count: agentRuns.length,
    top_idea_id: ranked[0]?.idea_id ?? null,
  };
}

function numberish(value) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) return 0;
  return Math.max(0, Math.min(100, parsed));
}

function renderMarkdown(artifact) {
  const lines = [];
  lines.push("# AI Agent Persona Panel");
  lines.push("");
  lines.push(`Generated: ${artifact.created_at}`);
  lines.push("");
  lines.push(
    "This is the AI-persona pass: each respondent below is one LLM call instructed to react as a specific person, not a deterministic score row.",
  );
  lines.push("");
  lines.push(`Model: \`${artifact.model}\``);
  lines.push(`Personas: \`${artifact.aggregate.persona_count}\``);
  lines.push(`Ideas evaluated per persona: \`${artifact.ideas.length}\``);
  lines.push("");
  lines.push("## Leaderboard");
  lines.push("");
  lines.push(
    "| Rank | Idea | AI Panel | #1 Votes | Mentions | Try | Share | Pay | Retain | Simulator |",
  );
  lines.push("| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");
  artifact.aggregate.ranked.forEach((idea, index) => {
    lines.push(
      `| ${index + 1} | ${idea.idea_name} | ${idea.ai_panel_score} | ${idea.first_place_votes} | ${idea.mentions} | ${idea.avg_try_intent} | ${idea.avg_share_intent} | ${idea.avg_pay_intent} | ${idea.avg_retention_intent} | ${idea.simulator_score} |`,
    );
  });
  lines.push("");
  lines.push("## Top Idea Notes");
  for (const idea of artifact.aggregate.ranked.slice(0, 5)) {
    lines.push("");
    lines.push(`### ${idea.idea_name}`);
    lines.push("");
    lines.push(`Score: \`${idea.ai_panel_score}\`; first-place votes: \`${idea.first_place_votes}\`.`);
    lines.push("");
    lines.push("Quotes:");
    for (const quote of idea.quotes.slice(0, 3)) lines.push(`- ${quote}`);
    lines.push("");
    lines.push("Objections:");
    for (const objection of idea.objections.slice(0, 3)) lines.push(`- ${objection}`);
  }
  lines.push("");
  lines.push("## Persona Summaries");
  for (const run of artifact.agent_runs) {
    lines.push("");
    lines.push(`### ${run.persona_name}`);
    lines.push("");
    lines.push(run.parsed?.first_person_summary ?? "");
    const top = run.parsed?.ranked_reactions?.[0];
    if (top) {
      lines.push("");
      lines.push(`Top pick: \`${top.idea_id}\` — ${top.why_it_hits}`);
    }
  }
  lines.push("");
  return `${lines.join("\n")}\n`;
}

function getPersonas() {
  return [
  {
    id: "maya-ugc-operator",
    name: "Maya",
    segment: "creator_operator",
    age: 27,
    context:
      "Runs a small TikTok Shop side hustle, makes product clips after work, watches creator economy YouTube, pays for tools if they save time.",
    budget: "$20-80/mo for tools that earn back money",
    habits: "TikTok, CapCut, Shopify, Instagram Reels, group chats with other sellers",
    skepticism: "Hates generic AI ads, worries about ad policy and fake-looking people.",
  },
  {
    id: "jules-style-social",
    name: "Jules",
    segment: "gen_z_social_creator",
    age: 21,
    context:
      "College student, posts outfit and friend-group content, uses AI filters casually but deletes anything that feels cringe.",
    budget: "$0-12/mo, buys packs for special moments",
    habits: "TikTok search, Instagram Stories, Pinterest, Snapchat, iMessage",
    skepticism: "Won't pay unless the output makes her look cooler or saves embarrassment.",
  },
  {
    id: "marco-dating-fatigued",
    name: "Marco",
    segment: "dating_app_fatigued",
    age: 32,
    context:
      "Single professional, tired of dating apps, wants better photos/prompts but does not want to feel fake.",
    budget: "$30-150 one-time if it clearly improves matches",
    habits: "Hinge, Instagram, group texts, gym, local events",
    skepticism: "Distrusts over-edited photos and anything that makes him sound like a pickup artist.",
  },
  {
    id: "tanya-parent-memory",
    name: "Tanya",
    segment: "parents_and_families",
    age: 38,
    context:
      "Parent with thousands of photos, shares privately with family, buys photo books before holidays.",
    budget: "$8-20/mo or $40-150 for a great gift",
    habits: "iMessage, Facebook groups, Instagram, Google Photos, school parent chats",
    skepticism: "Child privacy and weird AI faces are dealbreakers.",
  },
  {
    id: "devon-meme-native",
    name: "Devon",
    segment: "gen_z_social_creator",
    age: 24,
    context:
      "Lives in group chats and fandom/meme communities, makes jokes fast, wants tools that produce funny shareable artifacts in seconds.",
    budget: "$0-10/mo plus occasional pack buys",
    habits: "X, Discord, TikTok, Reddit, Instagram DMs",
    skepticism: "If the joke takes too long or looks branded, it dies.",
  },
  {
    id: "nina-fitness-progress",
    name: "Nina",
    segment: "habit_builder",
    age: 29,
    context:
      "Tracks workouts and runs, likes accountability challenges, shares progress but avoids performative hustle culture.",
    budget: "$10-25/mo for something she uses weekly",
    habits: "Strava, Apple Health, Instagram Stories, gym group chat",
    skepticism: "Does not want another streak app that nags her and then becomes guiltware.",
  },
  {
    id: "aarav-language-traveler",
    name: "Aarav",
    segment: "learning_travel",
    age: 35,
    context:
      "Travels for work, wants practical language confidence and would practice if scenarios felt real.",
    budget: "$15-30/mo",
    habits: "YouTube, Duolingo, WhatsApp, Google Maps, travel TikTok",
    skepticism: "Hates classroom-feeling exercises and robotic voice practice.",
  },
  {
    id: "bianca-pet-owner",
    name: "Bianca",
    segment: "pet_owner",
    age: 30,
    context:
      "Dog owner, posts pet clips weekly, buys silly pet merch, sends pet videos constantly.",
    budget: "$5-15/mo plus gift/merch purchases",
    habits: "TikTok, Instagram Reels, family group chat, Chewy",
    skepticism: "Novelty fades unless her dog becomes a repeat character.",
  },
  {
    id: "owen-indie-creator",
    name: "Owen",
    segment: "creator_operator",
    age: 41,
    context:
      "Makes a podcast/newsletter on nights and weekends, wants clips and assets but is allergic to complicated workflows.",
    budget: "$20-60/mo if it saves editing time",
    habits: "Substack, X, YouTube, Descript, Riverside",
    skepticism: "Will churn if the tool makes bland content that looks like everyone else's.",
  },
  {
    id: "sasha-shopping-dupes",
    name: "Sasha",
    segment: "value_shopper",
    age: 26,
    context:
      "Saves outfits, rooms, recipes, and products constantly, wants help making sense of screenshots and finding cheaper alternatives.",
    budget: "$0-10/mo, affiliate-driven is fine",
    habits: "TikTok search, Pinterest, Instagram saves, Amazon, resale apps",
    skepticism: "Does not trust tools that push sponsored junk or bad matches.",
  },
  {
    id: "leo-student-social",
    name: "Leo",
    segment: "student",
    age: 19,
    context:
      "Uses AI for studying and jokes, likes competitive mini-games, shares quiz results and funny cards with friends.",
    budget: "$0-8/mo",
    habits: "Discord, TikTok, Snapchat, Canvas, group chats",
    skepticism: "If it feels like homework, he bounces.",
  },
  {
    id: "rachel-home-food",
    name: "Rachel",
    segment: "busy_household",
    age: 34,
    context:
      "Busy household decision-maker, cooks at home, redecorates slowly, wants practical ideas that look good enough to share.",
    budget: "$8-20/mo or affiliate/free",
    habits: "Instagram, Pinterest, grocery apps, home decor TikTok, family texts",
    skepticism: "Unrealistic AI outputs annoy her. She needs doable, budget-aware help.",
  },
  ];
}

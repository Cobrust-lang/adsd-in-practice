<script lang="ts">
	// Wave M3 placeholder — Pub/Sub command set (SUBSCRIBE / UNSUBSCRIBE
	// / PUBLISH) lands in Wave M3 (cs01 CLAUDE.md §3 items 8). Until
	// then this is intentionally a stub so the nav has all three links
	// and M3 only swaps the page content. See top-level CLAUDE.md §1.3
	// "显式标 stub" — page copy below literally contains
	// "M3 placeholder" so it shows up in a grep.
</script>

<svelte:head>
	<title>Pub/Sub · mini-redis-rust</title>
</svelte:head>

<section class="space-y-4 max-w-2xl">
	<h1 class="text-2xl font-bold">Pub/Sub</h1>

	<div class="alert alert-warning">
		<span>
			This page is a <strong>M3 placeholder</strong>. Live subscriptions land in Wave M3.
		</span>
	</div>

	<div class="card bg-base-100 shadow">
		<div class="card-body">
			<h2 class="card-title text-base">Pub/Sub UI coming in Wave M3</h2>
			<p class="text-sm text-base-content/70">
				M2.2 ships the dashboard + keys live views off the existing
				<code class="badge badge-ghost font-mono">/api/stats</code>
				and
				<code class="badge badge-ghost font-mono">/api/keys</code>
				SSE streams (see ADR-0007). Pub/Sub state needs per-subscriber tracking
				(<code class="font-mono">SUBSCRIBE</code> / <code class="font-mono">UNSUBSCRIBE</code> /
				<code class="font-mono">PUBLISH</code>), which is the M3 scope.
			</p>
			<p class="text-sm text-base-content/70">
				When M3 lands this page becomes a live channel inspector — subscribe to a channel from
				the UI and see published messages stream in.
			</p>
			<div class="card-actions justify-end pt-2">
				<span class="badge badge-warning">stub</span>
				<span class="badge badge-outline font-mono">M3</span>
			</div>
		</div>
	</div>
</section>

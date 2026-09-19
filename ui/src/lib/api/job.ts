/**
 * The Job envelope, which is the same for every Job there will ever be.
 *
 * Asking for slow work answers a job id at once; state, progress and result are
 * read back through the ordinary Operations `get_job` and `list_jobs`
 * (ADR 0032). That shape is the one thing a Job's own declaration does not
 * describe — its declared output is its eventual result — so it is written here
 * once rather than generated per Operation. Everything else about a Job, its
 * states included, comes from the generated declarations.
 */

import type { GetJobOutput, KamosuClient } from './catalogue';

/** What asking for a Job answers, immediately, whatever the Job is. */
export interface JobAsk {
	job_id: string;
}

/** The states a Job never leaves once it reaches one. */
const ENDED = ['completed', 'failed', 'cancelled'] as const;

/**
 * Wait for one Job to reach an end state, by asking `get_job` — the ordinary
 * Operation, at the ordinary Door. There is no push channel and no second way
 * in: a Job's row is the truth about it, and polling that row is how every
 * caller reads one, an agent at the MCP door included (ADR 0032).
 *
 * A Job that failed is raised as the error it is, so a caller writes one happy
 * path rather than checking a status by hand and forgetting the other two.
 *
 * `whileWaiting` is handed each poll's answer, so a caller that wants to show
 * how far the work has got reads the same row every other caller reads. Some
 * Jobs are long — downloading a model is hundreds of megabytes — so
 * `giveUpAfter` is the caller's to set rather than a constant that fits the
 * shortest one.
 *
 * `stopped` lets a screen that has closed stop asking: the waiting ends with
 * the last answer read, and the Job itself carries on — only the reading of it
 * stops.
 */
export async function waitForJob(
	kamosu: KamosuClient,
	jobId: string,
	{
		every = 400,
		giveUpAfter = 120_000,
		whileWaiting,
		stopped,
	}: {
		every?: number;
		giveUpAfter?: number;
		whileWaiting?: (job: GetJobOutput) => void;
		stopped?: () => boolean;
	} = {},
): Promise<GetJobOutput> {
	const until = Date.now() + giveUpAfter;
	for (;;) {
		const job = await kamosu.getJob({ job_id: jobId });
		if ((ENDED as readonly string[]).includes(job.status)) {
			if (job.status === 'completed') return job;
			throw new Error(job.error ?? `the job ${job.status}`);
		}
		whileWaiting?.(job);
		if (stopped?.()) return job;
		if (Date.now() > until) throw new Error('the job is still running');
		await new Promise((wake) => setTimeout(wake, every));
	}
}

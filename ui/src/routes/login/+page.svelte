<script lang="ts">
	import { goto } from '$app/navigation';
	import LoginForm from '$lib/components/login-form.svelte';
	import { onMount } from 'svelte';

	let isLoading: boolean;
	let error: string | undefined;

	let conditionalAbortController: AbortController | undefined;

	onMount(async () => {
		const response = await fetch('/api/v1/auth/discoverable/start', {
			method: 'POST',
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		if (!response.ok) {
			console.error('Failed to start discoverable authentication:', response.status, response.statusText);
			return;
		}
		const { publicKey: publicKeyJSON, mediation } = await response.json() satisfies {
			publicKey: PublicKeyCredentialRequestOptionsJSON,
			mediation?: CredentialMediationRequirement,
		};
		console.debug({ publicKeyJSON, mediation });
		const publicKey = PublicKeyCredential.parseRequestOptionsFromJSON(publicKeyJSON);
		conditionalAbortController = new AbortController();
		console.log('starting discoverable authentication');
		const credential = await navigator.credentials.get({
			publicKey,
			mediation: 'conditional',
			signal: conditionalAbortController.signal,
		});
		conditionalAbortController = undefined;
		if (!credential) {
			console.warn('No discoverable passkey found');
			return;
		}
		if (!(credential instanceof PublicKeyCredential)) {
			error = 'Invalid passkey type';
			return;
		}

		// Complete authentication
		isLoading = true;
		const finish_response = await fetch('/api/v1/auth/discoverable/finish', {
			method: 'POST',
			body: JSON.stringify(credential.toJSON()),
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		if (!finish_response.ok) {
			error = 'Failed to log in: ' + (await finish_response.text());
			isLoading = false;
			return;
		}
		// FIXME: redirect to home page
		goto('/home');
	});

	async function handleLogin(event: SubmitEvent) {
		event.preventDefault();
		if (conditionalAbortController) {
			conditionalAbortController.abort();
			conditionalAbortController = undefined;
		}
		error = undefined;
		const formData = new FormData(event.target as HTMLFormElement);
		console.log(formData);
		const email = formData.get('email') as string;
		let response_promise = fetch('/api/v1/auth/start', {
			method: 'POST',
			body: JSON.stringify({ email }),
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		isLoading = true;
		const start_reponse = await response_promise;
		if (!start_reponse.ok) {
			if (start_reponse.status === 404) {
				error = 'User not found';
			} else {
				error = 'Failed to log in: ' + (await start_reponse.text());
			}
			isLoading = false;
			return;
		}

		let { publicKey, mediation } = (await start_reponse.json()) as {
			publicKey: PublicKeyCredentialRequestOptionsJSON;
			mediation?: CredentialMediationRequirement;
		};
		const parsedPublicKey = PublicKeyCredential.parseRequestOptionsFromJSON(publicKey);
		let credential: Credential | null;
		try {
			credential = await navigator.credentials.get({
				publicKey: parsedPublicKey,
				mediation: mediation
			});
		} catch (e) {
			isLoading = false;
			if (e instanceof DOMException && e.name === 'NotAllowedError') {
				error = 'Passkey operation was cancelled or was not allowed';
			} else {
				console.error("Passkey error:", e);
			}
			return;
		}
		if (!credential) {
			error = 'No passkey found';
			isLoading = false;
			return;
		}
		if (!(credential instanceof PublicKeyCredential)) {
			error = 'Invalid passkey type';
			isLoading = false;
			return;
		}

		const finish_response = await fetch('/api/v1/auth/finish', {
			method: 'POST',
			body: JSON.stringify(credential.toJSON()),
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		if (finish_response.ok) {
			let data = await finish_response.json();
			console.log(data);
			// FIXME: redirect to home page
			goto('/home');
		} else {
			error = 'Failed to login; please try again';
		}
	}
</script>

<div class="bg-background flex min-h-svh flex-col items-center justify-center gap-6 p-6 md:p-10">
	<div class="w-full max-w-sm">
		<LoginForm onsubmit={handleLogin} {isLoading} {error} />
	</div>
</div>

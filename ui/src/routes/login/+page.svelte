<script lang="ts">
	import LoginForm from '$lib/components/login-form.svelte';

	let isLoading: boolean;
	let error: string | undefined;

	async function handleLogin(event: SubmitEvent) {
		event.preventDefault();
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
				error = 'Failed to login: ' + (await start_reponse.text());
			}
			isLoading = false;
			return;
		}

		let { publicKey, mediation } = (await start_reponse.json()) as {
			publicKey: PublicKeyCredentialRequestOptionsJSON;
			mediation?: CredentialMediationRequirement;
		};
		const parsedPublicKey = PublicKeyCredential.parseRequestOptionsFromJSON(publicKey);
		const credential = await navigator.credentials.get({
			publicKey: parsedPublicKey,
			mediation: mediation
		});
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

<script lang="ts">
	import LoginForm from '$lib/components/login-form.svelte';

	let isLoading = false;
	let error: string | undefined;

	async function handleSubmit(event: SubmitEvent) {
		event.preventDefault();
		const formData = new FormData(event.target as HTMLFormElement);
		console.log(formData);
		const email = formData.get('email') as string;
		const displayName = formData.get('displayName') as string;
		isLoading = true;
		const start_response = await fetch('/api/v1/register/start', {
			method: 'POST',
			body: JSON.stringify({ email, display_name: displayName }),
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		if (!start_response.ok) {
			error = 'Failed to start registration: ' + (await start_response.text());
			isLoading = false;
			return;
		}
		const { publicKey } = (await start_response.json()) as {
			publicKey: PublicKeyCredentialCreationOptionsJSON;
		};
		console.debug({ publicKey });
        publicKey.authenticatorSelection!.requireResidentKey = true;
        publicKey.authenticatorSelection!.residentKey = 'required';
		const parsedPublicKey = PublicKeyCredential.parseCreationOptionsFromJSON(publicKey);
		const credential = await navigator.credentials.create({
			publicKey: parsedPublicKey
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

		const finish_response = await fetch('/api/v1/register/finish', {
			method: 'POST',
			body: JSON.stringify({
				user: {
					email,
					display_name: displayName
				},
				passkey: credential.toJSON()
			}),
			headers: {
				'Content-Type': 'application/json'
			},
			credentials: 'include'
		});
		if (!finish_response.ok) {
			error = 'Failed to finish registration: ' + (await finish_response.text());
			isLoading = false;
			return;
		}
		// FIXME: Redirect to home page
		window.location.href = '/logout';
	}
</script>

<div class="bg-background flex min-h-svh flex-col items-center justify-center gap-6 p-6 md:p-10">
	<div class="w-full max-w-sm">
		<LoginForm register onsubmit={handleSubmit} {isLoading} {error} />
	</div>
</div>

import { useState, useEffect } from 'react';
import { Button } from './ui/button';
import { listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-shell';

interface UserData {
    id: string;
    username: string;
    avatar_url?: string;
}

export const Test = () => {
    const [isLoggedIn, setIsLoggedIn] = useState(false);
    const [userData, setUserData] = useState<UserData | null>(null);
    const [error, setError] = useState<string | null>(null);
    const [isLoading, setIsLoading] = useState(true);

    // Effect for auth initialization and listening to backend events
    useEffect(() => {
        console.log('Auth effect mounted.');
        let unlisten: (() => void) | undefined;

        const initializeAuth = async () => {
            console.log('Initializing authentication...');
            try {
                // Listen for auth state changes from the Rust backend
                unlisten = await listen<boolean>('auth-state-changed', (event) => {
                    console.log(`[EVENT] 'auth-state-changed' received. Payload: ${event.payload}`);
                    setIsLoggedIn(event.payload);
                    if (!event.payload) {
                        console.log('User logged out, clearing user data.');
                        setUserData(null); // Clear user data on logout
                    }
                });
                console.log("Listener for 'auth-state-changed' is active.");

                // Check the initial authentication status
                console.log("Invoking 'check_auth_status' for initial status...");
                const initialStatus = await invoke<boolean>('check_auth_status');
                console.log(`Initial auth status is: ${initialStatus}`);
                setIsLoggedIn(initialStatus);
            } catch (err) {
                console.error('Failed during auth initialization:', err);
                setError('A critical error occurred while setting up authentication.');
            } finally {
                console.log('Auth initialization finished.');
                setIsLoading(false);
            }
        };

        initializeAuth();

        return () => {
            console.log('Auth effect unmounted. Cleaning up listener.');
            unlisten?.();
        };
    }, []);

    // Effect to fetch user data when the user is logged in
    useEffect(() => {
        console.log(`User data effect triggered. isLoggedIn: ${isLoggedIn}`);
        if (isLoggedIn) {
            console.log("User is logged in. Invoking 'get_me' to fetch user data...");
            invoke<UserData>('get_me')
                .then((data) => {
                    console.log('Successfully fetched user data:', data);
                    setUserData(data);
                })
                .catch((err) => {
                    console.error('Failed to fetch user data:', err);
                    setError('Your session might be invalid. Please try logging in again.');
                    setIsLoggedIn(false); // Token is likely invalid, force logout
                });
        } else {
            console.log('User is not logged in, skipping fetch for user data.');
        }
    }, [isLoggedIn]);

    const loginWithGitHub = async () => {
        console.log('loginWithGitHub button clicked.');
        setError(null);
        const authUrl = `http://127.0.0.1:8080/auth/github?client=tauri`;
        try {
            console.log(`Opening external auth URL: ${authUrl}`);
            await open(authUrl);
        } catch (e) {
            console.error('Failed to open external browser:', e);
            setError('Could not open the login page. Please check your browser settings.');
        }
    };

    const handleLogout = async () => {
        console.log('handleLogout button clicked.');
        try {
            console.log("Invoking 'logout' command...");
            await invoke('logout');
            console.log("'logout' command successful. The 'auth-state-changed' listener will handle UI updates.");
        } catch (err) {
            console.error('Logout failed:', err);
            setError('An error occurred during logout.');
        }
    };

    console.log(`Component rendering with state: isLoading=${isLoading}, isLoggedIn=${isLoggedIn}, error=${!!error}`);

    if (isLoading) {
        console.log('Rendering: Loading state');
        return <div>Loading...</div>;
    }

    if (error) {
        console.log('Rendering: Error state');
        return (
            <div>
                <h1>Application Error</h1>
                <p className='text-red-600'>{error}</p>
                <Button onClick={loginWithGitHub}>Try Again</Button>
            </div>
        );
    }

    if (isLoggedIn) {
        console.log('Rendering: Logged-in state');
        return (
            <div>
                <h1>Welcome Back!</h1>
                {userData ? (
                    <div>
                        <p>
                            Logged in as: <strong>{userData.username}</strong>
                        </p>
                        {userData.avatar_url && (
                            <img
                                src={userData.avatar_url}
                                alt='User avatar'
                                style={{ width: 50, borderRadius: '50%' }}
                            />
                        )}
                    </div>
                ) : (
                    <p>Loading your profile...</p>
                )}
                <Button onClick={handleLogout}>Logout</Button>
            </div>
        );
    }

    console.log('Rendering: Logged-out (default) state');
    return (
        <div>
            <h1>Welcome to WayClip</h1>
            <p>Please log in with GitHub to continue.</p>
            <Button onClick={loginWithGitHub}>Login with GitHub</Button>
        </div>
    );
};

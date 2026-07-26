#!/usr/bin/perl
# A faithful extract of Pixiewood's `ParallelRunner`, the helper that
# `pixiewood build` uses to run one `ninja` per Android ABI.
#
# It is copied verbatim from gtk-android-builder at the revision pinned in
# `build-aux/android/Containerfile` (`PIXIEWOOD_COMMIT`), so that the failure
# it produces here is exactly the one a real build produces.
#
# Usage: parallel-runner.pl COMMAND [ARGUMENT ...]

use strict;
use warnings;

package ParallelRunner;

sub new {
	my ($class) = @_;
	my $self = {
		pids => [],
	};
	bless $self, $class;
	return $self;
}

sub launch {
	my ($self, @cmd) = @_;

	my $pid = fork;
	die "fork failed: $!" if not defined $pid;

	if ($pid == 0) {
		exec @cmd or die "exec failed: $!";
	} else {
		push @{$self->{pids}}, $pid;
	}
}

sub wait {
	my ($self) = @_;

	while (scalar @{$self->{pids}} > 0) {
		my $child = waitpid(-1, 0);
		my $status = $?;
		die "Unexpected child process $child died" unless (grep { $_ == $child } @{$self->{pids}});

		if ($status != 0) {
			foreach my $pid (@{$self->{pids}}) {
				kill QUIT => $pid;
			}
			return 0;
		}
		@{$self->{pids}} = grep { !/$child/ } @{$self->{pids}};
	}

	1;
}

package main;

my $builds = ParallelRunner->new;
$builds->launch(@ARGV);
die "Ninja build failed, see logs for more info" unless $builds->wait();
print "build succeeded\n";

-module(zlib_ffi).
-export([zlib_safeInflate/2]).

% Wraps zlib:safeInflate to return a valid Gleam Result() type and to turn all
% errors into an Error(Nil).
%
zlib_safeInflate(Z, Data) ->
    try
        case zlib:safeInflate(Z, Data) of
            {continue, Output} -> {ok, {continue, list_to_bitstring(Output)}};
            {finished, Output} -> {ok, {finished, list_to_bitstring(Output)}};
            {need_dictionary, _, _} -> {error, nil}
        end
    catch
        error:_ -> {error, nil}
    end.
